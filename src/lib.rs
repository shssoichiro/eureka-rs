#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate itertools;
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
mod instance;
mod registry;
mod rest;
mod resolver;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use reqwest::{Error as ReqwestError, Method, Response, StatusCode};
use serde_json::{Map, Number, Value};
use serde_yaml::Value as YamlValue;

use self::instance::InstanceClient;
use self::registry::RegistryClient;
use self::rest::EurekaRestClient;

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

#[derive(Debug)]
pub struct EurekaClient {
    config: HashMap<String, Value>,
    client: EurekaRestClient,
    registry: RegistryClient,
    instance: Option<InstanceClient>,
}

impl EurekaClient {
    pub fn new(env: &str, mut config: HashMap<String, Value>) -> Result<Self, EurekaError> {
        let filename = config
            .get(&String::from("filename"))
            .and_then(|filename| filename.as_str())
            .map(String::from)
            .unwrap_or_else(|| "eureka-client".into());
        let default_yaml = match load_yaml(format!("{}.yml", filename)) {
            Ok(yaml) => yaml,
            Err(EurekaError::FileNotFound) => HashMap::new(),
            Err(e) => {
                return Err(e);
            }
        };
        let env_yaml = match load_yaml(format!("{}-{}.yml", filename, env)) {
            Ok(yaml) => yaml,
            Err(EurekaError::FileNotFound) => HashMap::new(),
            Err(e) => {
                return Err(e);
            }
        };
        for (key, prop) in env_yaml
            .into_iter()
            .chain(default_yaml.into_iter())
            .chain((*DEFAULT_CONFIG).clone().into_iter())
        {
            config.entry(key).or_insert(prop);
        }
        Self::validate_config(&config)?;

        let base_url = {
            let eureka_cfg = config["eureka"].as_object().unwrap();
            let ssl = eureka_cfg["ssl"].as_bool().unwrap_or(false);
            let protocol = if ssl { "https" } else { "http" };
            let host = eureka_cfg["host"].as_str().unwrap();
            let port = eureka_cfg["port"].as_u64().unwrap();
            let service_path = eureka_cfg["servicePath"].as_str().unwrap();
            format!("{}://{}:{}{}", protocol, host, port, service_path)
        };
        Ok(EurekaClient {
            client: EurekaRestClient::new(base_url.clone()),
            registry: RegistryClient::new(base_url.clone()),
            instance: if config["eureka"]
                .get("registerWithEureka")
                .and_then(|reg| reg.as_bool())
                .unwrap_or(false)
            {
                Some(InstanceClient::new(
                    base_url,
                    serde_json::from_value(config["instance"].clone())
                        .map_err(|e| EurekaError::ParseError(e.to_string()))?,
                ))
            } else {
                None
            },
            config,
        })
    }

    pub fn start(&mut self) {
        self.registry.start();
        if let Some(ref instance) = self.instance {
            instance.start();
        }
    }

    pub fn make_request<V: Into<Value>>(
        &self,
        app: &str,
        path: &str,
        method: &Method,
        body: &V,
    ) -> Result<Response, EurekaError> {
        unimplemented!()
    }

    fn validate_config(config: &HashMap<String, Value>) -> Result<(), EurekaError> {
        let validate = |namespace: &str, key: &str| -> Result<(), EurekaError> {
            if config
                .get(namespace)
                .and_then(|ns| ns.as_object())
                .map(|ns| ns.contains_key(key))
                .unwrap_or(false)
            {
                return Ok(());
            };
            Err(EurekaError::Configuration(format!(
                "Missing \"{}.{}\" config value.",
                namespace, key
            )))
        };

        validate("eureka", "host")?;
        validate("eureka", "port")?;
        validate("eureka", "servicePath")?;
        if config
            .get("eureka")
            .and_then(|eureka| eureka.get("registerWithEureka"))
            .and_then(|val| val.as_bool())
            .unwrap_or(false)
        {
            validate("instance", "app")?;
            validate("instance", "vipAddress")?;
            validate("instance", "port")?;
            validate("instance", "dataCenterInfo")?;
        }

        Ok(())
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
