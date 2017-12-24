use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::thread;
use std::time::Duration;

use reqwest::Method;
use serde_json::{self, Value};
use serde_yaml;

use {EurekaError, DEFAULT_CONFIG};
use aws::AwsMetadata;
use register::{Instance, Registry};
use resolver::*;

fn load_yaml<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Value>, EurekaError> {
    Ok(
        serde_yaml::from_reader(File::open(path).map_err(|_| EurekaError::FileNotFound)?)
            .map_err(|_| EurekaError::ParseError)?,
    )
}

#[derive(Debug)]
pub struct EurekaClient {
    config: HashMap<String, Value>,
    metadata_client: Option<AwsMetadata>,
    cluster_resolver: Box<ClusterResolver>,
    cache: EurekaCache,
    registry_fetch_active: bool,
    heartbeat_active: bool,
}

impl EurekaClient {
    pub fn new(env: &str, mut config: HashMap<String, Value>) -> Result<Self, EurekaError> {
        let filename = config
            .get(&String::from("filename"))
            .cloned()
            .unwrap_or_else(|| Value::String(String::from("eureka-client")))
            .as_str()
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
            if !config.contains_key(&key) {
                config.insert(key, prop);
            }
        }
        Self::validate_config(&config)?;
        let mut client = EurekaClient {
            config: config.clone(),
            metadata_client: None,
            cluster_resolver: if config.get("eureka").map_or(false, |eureka| {
                eureka.as_object().map_or(false, |eureka| {
                    eureka.get(&String::from("useDns")).is_some()
                })
            }) {
                Box::new(DnsClusterResolver::new(config))
            } else {
                Box::new(ConfigClusterResolver::new(config))
            },
            cache: EurekaCache::default(),
            registry_fetch_active: false,
            heartbeat_active: false,
        };
        if client.is_amazon_datacenter() {
            client.metadata_client = Some(AwsMetadata::default());
        }
        Ok(client)
    }

    fn instance_id(&self) -> &str {
        let instance = self.config[&String::from("instance")].as_object().unwrap();
        if let Some(instance_id) = instance.get(&String::from("instanceId")) {
            return instance_id.as_str().unwrap();
        }
        if self.is_amazon_datacenter() {
            return instance
                .get(&String::from("dataCenterInfo"))
                .unwrap()
                .get(&String::from("metadata"))
                .unwrap()
                .get(&String::from("instance-id"))
                .unwrap()
                .as_str()
                .unwrap();
        }
        instance
            .get(&String::from("hostName"))
            .unwrap()
            .as_str()
            .unwrap()
    }

    fn is_amazon_datacenter(&self) -> bool {
        let instance_value = match self.config.get("instance") {
            Some(x) => x,
            None => {
                return false;
            }
        };
        let instance = match instance_value.as_object() {
            Some(x) => x,
            None => {
                return false;
            }
        };
        let name_value = match instance.get(&"name".to_string()) {
            Some(x) => x,
            None => {
                return false;
            }
        };
        match name_value.as_str() {
            Some(x) => x == "amazon",
            None => false,
        }
    }

    fn start<F: Fn()>(&mut self) -> Result<(), EurekaError> {
        if self.metadata_client.is_some()
            && self.config[&String::from("eureka")]
                .get(&String::from("fetchMetadata"))
                .unwrap_or_else(|| &Value::Bool(false))
                .as_bool()
                .unwrap()
        {
            self.add_instance_metadata();
        }
        if self.config[&String::from("eureka")]
            .get(&String::from("registerWithEureka"))
            .unwrap_or_else(|| &Value::Bool(false))
            .as_bool()
            .unwrap()
        {
            self.register();
            self.start_heartbeats();
        }
        if self.config[&String::from("eureka")]
            .get(&String::from("fetchRegistry"))
            .unwrap_or_else(|| &Value::Bool(false))
            .as_bool()
            .unwrap()
        {
            self.start_registry_fetches();
            if self.config[&String::from("eureka")]
                .get(&String::from("waitForRegistry"))
                .unwrap_or_else(|| &Value::Bool(false))
                .as_bool()
                .unwrap()
            {
                self.wait_for_registry_update()?;
            } else {
                self.fetch_registry();
            }
        }
        Ok(())
    }

    fn wait_for_registry_update(&self) -> Result<(), EurekaError> {
        self.fetch_registry();
        loop {
            let instances = self.get_instances_by_vip_address(
                self.config["instance"]
                    .get(&String::from("vipAddress"))
                    .map_or("", |i| i.as_str().unwrap()),
            );
            if instances.is_empty() {
                thread::sleep(Duration::from_secs(2));
            } else {
                break;
            }
        }
        Ok(())
    }

    fn stop(&mut self) {
        self.registry_fetch_active = false;
        if self.config[&String::from("eureka")]
            .get(&String::from("registerWithEureka"))
            .unwrap_or_else(|| &Value::Bool(false))
            .as_bool()
            .unwrap()
        {
            self.heartbeat_active = false;
            self.deregister();
        }
    }

    fn validate_config(config: &HashMap<String, Value>) -> Result<(), EurekaError> {
        let validate = |namespace: String, key: String| -> Result<(), EurekaError> {
            match config.get(&namespace) {
                Some(ns) => match ns.as_object() {
                    Some(ns) if ns.contains_key(&key) => {
                        return Ok(());
                    }
                    _ => {}
                },
                _ => {}
            };
            Err(EurekaError::Configuration(format!(
                "Missing \"{}.{}\" config value.",
                namespace, key
            )))
        };

        if config
            .get("eureka")
            .and_then(|eureka| eureka.get("registerWithEureka"))
            .and_then(|val| val.as_bool())
            .unwrap_or(false)
        {
            validate("instance".into(), "app".into())?;
            validate("instance".into(), "vipAddress".into())?;
            validate("instance".into(), "port".into())?;
            validate("instance".into(), "dataCenterInfo".into())?;
        }

        Ok(())
    }

    fn register(&mut self) {
        *self.config
            .get_mut(&String::from("instance"))
            .unwrap()
            .as_object_mut()
            .unwrap()
            .entry(String::from("status"))
            .or_insert_with(|| Value::String(String::new())) = Value::String(String::from("UP"));
        let instance = self.config[&String::from("instance")].as_object().unwrap();
        let uri = instance.get("app").unwrap().as_str().unwrap().to_owned();
        let mut body = HashMap::with_capacity(1);
        body.insert("instance", instance);
        self.eureka_request(
            &EurekaRequestConfig {
                method: Method::Post,
                uri,
                body: Some(serde_json::to_value(body).unwrap()),
            },
            0,
        )
    }

    fn deregister(&self) {
        unimplemented!()
    }

    fn start_heartbeats(&self) {
        unimplemented!()
    }

    fn renew(&self) {
        unimplemented!()
    }

    fn start_registry_fetches(&self) {
        unimplemented!()
    }

    fn get_instances_by_app_id(&self, app_id: &str) -> Vec<Instance> {
        unimplemented!()
    }

    fn get_instances_by_vip_address(&self, vip_address: &str) -> Vec<Instance> {
        unimplemented!()
    }

    fn fetch_registry(&self) -> Registry {
        unimplemented!()
    }

    fn transform_registry(&self, registry: Registry) {
        unimplemented!()
    }

    fn transform_app(&self, app: &str, cache: EurekaCache) {
        unimplemented!()
    }

    fn validate_instance(&self, instance: Instance) -> bool {
        unimplemented!()
    }

    fn split_vip_address(vip_address: &str) -> Vec<&str> {
        unimplemented!()
    }

    fn add_instance_metadata(&self) {
        unimplemented!()
    }

    fn eureka_request(&self, opts: &EurekaRequestConfig, retry_attempts: usize) {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
struct EurekaCache {
    app: HashMap<String, Value>,
    vip: HashMap<String, Value>,
}

impl Default for EurekaCache {
    fn default() -> Self {
        EurekaCache {
            app: HashMap::new(),
            vip: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct EurekaRequestConfig {
    pub method: Method,
    pub uri: String,
    pub body: Option<Value>,
}
