use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use reqwest::{Method, Response, Result as ReqwestResult, StatusCode};
use serde_json::{self, Value};
use serde_yaml;

use {EurekaError, DEFAULT_CONFIG};
use aws::AwsMetadata;
use register::{Instance, RegisterData, Registry};
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
    registry_fetch_active: Arc<AtomicBool>,
    registry_client: Arc<RegistryClient>,
    heartbeat_active: Arc<AtomicBool>,
    instance_client: Arc<InstanceClient>,
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
            config.entry(key).or_insert(prop);
        }
        Self::validate_config(&config)?;
        Self::mark_as_up(&mut config);
        let mut client = EurekaClient {
            config: config.clone(),
            metadata_client: None,
            cluster_resolver: if config
                .get("eureka")
                .and_then(|eureka| eureka.as_object())
                .and_then(|eureka| eureka.get(&String::from("useDns")))
                .and_then(|dns| dns.as_bool())
                .unwrap_or(false)
            {
                Box::new(DnsClusterResolver::new(&config))
            } else {
                Box::new(ConfigClusterResolver::new(&config))
            },
            registry_fetch_active: Arc::new(AtomicBool::new(false)),
            registry_client: Arc::new(RegistryClient::new()),
            heartbeat_active: Arc::new(AtomicBool::new(false)),
            instance_client: Arc::new(InstanceClient::new(
                serde_json::from_value(config[&String::from("instance")].clone()).unwrap(),
            )),
        };
        if client.instance_client.is_amazon_datacenter() {
            client.metadata_client = Some(AwsMetadata::new(&config));
        }
        Ok(client)
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
            self.instance_client.register()?;
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
                self.registry_client.fetch_registry()?;
            }
        }
        Ok(())
    }

    fn wait_for_registry_update(&self) -> Result<(), EurekaError> {
        self.registry_client.fetch_registry()?;
        loop {
            let instances = self.registry_client.get_instances_by_vip_address(
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

    fn stop(&mut self) -> Result<(), EurekaError> {
        self.registry_fetch_active.store(false, Ordering::Relaxed);
        if self.config[&String::from("eureka")]
            .get(&String::from("registerWithEureka"))
            .unwrap_or_else(|| &Value::Bool(false))
            .as_bool()
            .unwrap()
        {
            self.heartbeat_active.store(false, Ordering::Relaxed);
            self.instance_client.deregister()?;
        }
        Ok(())
    }

    fn validate_config(config: &HashMap<String, Value>) -> Result<(), EurekaError> {
        let validate = |namespace: String, key: String| -> Result<(), EurekaError> {
            if let Some(ns) = config.get(&namespace) {
                match ns.as_object() {
                    Some(ns) if ns.contains_key(&key) => {
                        return Ok(());
                    }
                    _ => {}
                }
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

    fn mark_as_up(config: &mut HashMap<String, Value>) {
        *config
            .get_mut(&String::from("instance"))
            .unwrap()
            .as_object_mut()
            .unwrap()
            .entry(String::from("status"))
            .or_insert_with(|| Value::String(String::new())) = Value::String(String::from("UP"));
    }

    fn start_heartbeats(&self) {
        self.heartbeat_active.store(true, Ordering::Relaxed);
        let interval = self.config[&String::from("eureka")]
            .as_object()
            .unwrap()
            .get("heartbeatInterval")
            .unwrap()
            .as_u64()
            .unwrap();
        let heartbeat = Arc::clone(&self.heartbeat_active);
        let instance_client = Arc::clone(&self.instance_client);
        thread::spawn(move || {
            while heartbeat.load(Ordering::Relaxed) {
                instance_client.renew();
                thread::sleep(Duration::from_millis(interval));
            }
        });
    }

    fn start_registry_fetches(&self) {
        self.registry_fetch_active.store(true, Ordering::Relaxed);
        let interval = self.config[&String::from("eureka")]
            .as_object()
            .unwrap()
            .get("registryFetchInterval")
            .unwrap()
            .as_u64()
            .unwrap();
        let registry_fetch = Arc::clone(&self.registry_fetch_active);
        let registry_client = Arc::clone(&self.registry_client);
        thread::spawn(move || {
            while registry_fetch.load(Ordering::Relaxed) {
                let _ = registry_client.fetch_registry();
                thread::sleep(Duration::from_millis(interval));
            }
        });
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
}

impl Drop for EurekaClient {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Debug)]
struct InstanceClient {
    instance: HashMap<String, Value>,
}

impl InstanceClient {
    pub fn new(instance: HashMap<String, Value>) -> Self {
        InstanceClient { instance }
    }

    pub fn register(&self) -> Result<(), EurekaError> {
        let uri = self.instance["app"].as_str().unwrap().to_owned();
        let mut body = HashMap::with_capacity(1);
        body.insert("instance", &self.instance);
        let response = eureka_request(
            &EurekaRequestConfig {
                method: Method::Post,
                uri,
                body: Some(serde_json::to_value(body).unwrap()),
            },
            0,
        ).and_then(Response::error_for_status);
        match response {
            Ok(_) => {
                info!(
                    "Registered with eureka: {}/{}",
                    self.instance["app"].as_str().unwrap(),
                    self.instance_id()
                );
                Ok(())
            }
            Err(e) => {
                warn!("Error registering with eureka: {}", e);
                Err(EurekaError::Network(e))
            }
        }
    }

    pub fn renew(&self) {
        let response = eureka_request(
            &EurekaRequestConfig {
                method: Method::Put,
                uri: format!(
                    "{}/{}",
                    self.instance["app"].as_str().unwrap(),
                    self.instance_id()
                ),
                body: None,
            },
            0,
        );
        match response {
            Ok(mut res) => {
                if res.status() == StatusCode::NotFound {
                    warn!("Eureka heartbeat failed, re-registering app");
                    let _ = self.register();
                } else if !res.status().is_success() {
                    warn!(
                        "Eureka heartbeat failed, will retry. Status code: {}, body: {:?}",
                        res.status(),
                        res.text()
                    );
                } else {
                    debug!("Eureka heartbeat success");
                }
            }
            Err(e) => {
                error!("An error in the request occurred: {}", e);
            }
        };
    }

    pub fn deregister(&self) -> Result<(), EurekaError> {
        let response = eureka_request(
            &EurekaRequestConfig {
                method: Method::Delete,
                uri: format!(
                    "{}/{}",
                    self.instance["app"].as_str().unwrap(),
                    self.instance_id()
                ),
                body: None,
            },
            0,
        ).and_then(Response::error_for_status);
        match response {
            Ok(_) => {
                info!(
                    "De-registered with eureka: {}/{}",
                    self.instance["app"].as_str().unwrap(),
                    self.instance_id()
                );
                Ok(())
            }
            Err(e) => {
                warn!("Error deregistering with eureka: {}", e);
                Err(EurekaError::Network(e))
            }
        }
    }

    fn instance_id(&self) -> String {
        if let Some(instance_id) = self.instance.get(&String::from("instanceId")) {
            return instance_id.as_str().unwrap().into();
        }
        if self.is_amazon_datacenter() {
            return self.instance[&String::from("dataCenterInfo")]
                .get(&String::from("metadata"))
                .unwrap()
                .get(&String::from("instance-id"))
                .unwrap()
                .as_str()
                .unwrap()
                .into();
        }
        self.instance[&String::from("hostName")]
            .as_str()
            .unwrap()
            .into()
    }

    pub fn is_amazon_datacenter(&self) -> bool {
        self.instance
            .get(&"name".to_string())
            .and_then(|name| Some(name.as_str().map(|s| s == "amazon").unwrap_or(false)))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
struct RegistryClient {
    cache: Mutex<EurekaCache>,
}

impl RegistryClient {
    pub fn new() -> Self {
        RegistryClient {
            cache: Mutex::new(EurekaCache::default()),
        }
    }

    pub fn fetch_registry(&self) -> Result<(), EurekaError> {
        let response = eureka_request(
            &EurekaRequestConfig {
                method: Method::Get,
                uri: String::new(),
                body: None,
            },
            0,
        ).and_then(Response::error_for_status);
        match response {
            Ok(mut resp) => {
                debug!("Retrieved registry successfully");
                self.transform_registry(resp.json().map_err(|_| {
                    EurekaError::UnexpectedState("Failed to parse registry response.".into())
                })?);
                Ok(())
            }
            Err(e) => {
                warn!("Error fetching registry: {}", e);
                Err(EurekaError::Network(e))
            }
        }
    }

    fn transform_registry(&self, registry: Registry) {
        let mut cache = EurekaCache::default();
        if registry.applications.application.is_array() {
            let apps: Vec<RegisterData> =
                serde_json::from_value(registry.applications.application.clone()).unwrap();
            for app in apps {
                self.transform_app(app, &mut cache);
            }
        } else {
            let app: RegisterData =
                serde_json::from_value(registry.applications.application.clone()).unwrap();
            self.transform_app(app, &mut cache);
        }
        *self.cache.lock().unwrap() = cache;
    }

    fn transform_app(&self, app: RegisterData, cache: &mut EurekaCache) {
        unimplemented!()
    }

    pub fn get_instances_by_app_id(&self, app_id: &str) -> Vec<Instance> {
        let instances: Vec<Instance> = serde_json::from_value(
            self.cache.lock().unwrap().app[&app_id.to_uppercase()].clone(),
        ).unwrap_or_default();
        if instances.is_empty() {
            warn!("Unable to retrieve instances for app ID: {}", app_id);
        }
        instances
    }

    pub fn get_instances_by_vip_address(&self, vip_address: &str) -> Vec<Instance> {
        let instances: Vec<Instance> = serde_json::from_value(
            self.cache.lock().unwrap().vip[vip_address].clone(),
        ).unwrap_or_default();
        if instances.is_empty() {
            warn!(
                "Unable to retrieve instances for vip address: {}",
                vip_address
            );
        }
        instances
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

fn eureka_request(opts: &EurekaRequestConfig, retry_attempts: usize) -> ReqwestResult<Response> {
    unimplemented!()
}
