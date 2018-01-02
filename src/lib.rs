#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate percent_encoding;
#[macro_use]
extern crate quick_error;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;

mod aws;
mod rest;
mod resolver;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use reqwest::{StatusCode, Error as ReqwestError};
use serde_json::{Map, Number, Value};
use serde_yaml::Value as YamlValue;

lazy_static! {
    static ref DEFAULT_CONFIG: HashMap<String, Value> = {
        let mut eureka = HashMap::with_capacity(15);
        eureka.insert(String::from("heartbeatInterval"), Value::Number(Number::from(30_000)));
        eureka.insert(String::from("registryFetchInterval"), Value::Number(Number::from(30_000)));
        eureka.insert(String::from("maxRetries"), Value::Number(Number::from(3)));
        eureka.insert(String::from("requestRetryDelay"), Value::Number(Number::from(500)));
        eureka.insert(String::from("fetchRegistry"), Value::Bool(true));
        eureka.insert(String::from("filterUpInstances"), Value::Bool(true));
        eureka.insert(String::from("servicePath"), Value::String(String::from("/eureka/v2/apps/")));
        eureka.insert(String::from("ssl"), Value::Bool(false));
        eureka.insert(String::from("useDns"), Value::Bool(false));
        eureka.insert(String::from("preferSameZone"), Value::Bool(true));
        eureka.insert(String::from("clusterRefreshInterval"), Value::Number(Number::from(300_000)));
        eureka.insert(String::from("fetchMetadata"), Value::Bool(true));
        eureka.insert(String::from("registerWithEureka"), Value::Bool(true));
        eureka.insert(String::from("useLocalMetadata"), Value::Bool(false));
        eureka.insert(String::from("preferIpAddress"), Value::Bool(false));
        let mut config = HashMap::with_capacity(2);
        config.insert(String::from("eureka"), Value::Object(eureka.into_iter().collect()));
        config.insert(String::from("instance"), Value::Object(Map::new()));
        config
    };
}

quick_error! {
    #[derive(Debug)]
    pub enum EurekaError {
        Network(err: ReqwestError) {
            description(err.description())
            cause(err)
        }
        Request(status: StatusCode) {
            description(status.canonical_reason().unwrap_or("Unknown Status Code"))
        }
        UnexpectedState(description: String) {
            description(description)
        }
        Configuration(description: String) {
            description(description)
        }
        FileNotFound {}
        ParseError(description: String) {}
    }
}

fn load_yaml<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Value>, EurekaError> {
    Ok(
        serde_yaml::from_reader(File::open(path).map_err(|_| EurekaError::FileNotFound)?)
            .map_err(|e| EurekaError::ParseError(e.to_string()))?,
    )
}

/// Maps in `serde_yaml` are more annoying to work with than in `serde_json` because they have
/// `Value` keys instead of `String` keys, so we're going to consistently use `serde_json`
/// throughout the library
fn map_yaml_to_json(yaml: YamlValue) -> Value {
    match yaml {
        YamlValue::Null => Value::Null,
        YamlValue::Bool(bool) => Value::Bool(bool),
        YamlValue::Number(ref number) if number.is_u64() => {
            Value::Number(Number::from(number.as_u64().unwrap()))
        }
        YamlValue::Number(ref number) if number.is_i64() => {
            Value::Number(Number::from(number.as_i64().unwrap()))
        }
        YamlValue::Number(ref number) if number.is_f64() => Value::Number(
            Number::from_f64(number.as_f64().unwrap())
                .unwrap_or_else(|| Number::from_f64(0.0).unwrap()),
        ),
        YamlValue::Number(_) => unreachable!(),
        YamlValue::String(str) => Value::String(str),
        YamlValue::Sequence(seq) => Value::Array(seq.into_iter().map(map_yaml_to_json).collect()),
        YamlValue::Mapping(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k.as_str().unwrap().to_string(), map_yaml_to_json(v)))
                .collect(),
        ),
    }
}

fn path_segment_encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::PATH_SEGMENT_ENCODE_SET)
        .to_string()
}

fn query_encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::QUERY_ENCODE_SET).to_string()
}
