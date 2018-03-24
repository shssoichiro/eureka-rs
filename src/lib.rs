extern crate itertools;
#[macro_use]
extern crate log;
extern crate percent_encoding;
#[macro_use]
extern crate quick_error;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub use reqwest::{Error as ReqwestError, Method, Response, StatusCode};
use reqwest::{Client as ReqwestClient, mime};
use reqwest::header::{Accept, qitem};
pub use self::instance::{Instance, PortData, StatusType};
use self::instance::InstanceClient;
use self::registry::RegistryClient;
use serde::Serialize;

mod aws;
mod instance;
mod registry;
mod rest;
mod resolver;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EurekaConfig {
    pub host: String,
    pub port: u16,
    pub heartbeat_interval: usize,
    pub registry_fetch_interval: usize,
    pub max_retries: usize,
    pub request_retry_delay: usize,
    pub fetch_registry: bool,
    pub filter_up_instances: bool,
    pub service_path: String,
    pub ssl: bool,
    pub use_dns: bool,
    pub prefer_same_zone: bool,
    pub cluster_refresh_interval: usize,
    pub fetch_metadata: bool,
    pub register_with_eureka: bool,
    pub use_local_metadata: bool,
    pub prefer_ip_address: bool,
}

impl Default for EurekaConfig {
    fn default() -> Self {
        EurekaConfig {
            host: "localhost".to_string(),
            port: 8761,
            heartbeat_interval: 30_000,
            registry_fetch_interval: 30_000,
            max_retries: 3,
            request_retry_delay: 500,
            fetch_registry: true,
            filter_up_instances: true,
            service_path: "/eureka".to_string(),
            ssl: false,
            use_dns: false,
            prefer_same_zone: true,
            cluster_refresh_interval: 300_000,
            fetch_metadata: true,
            register_with_eureka: true,
            use_local_metadata: false,
            prefer_ip_address: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseConfig {
    pub eureka: EurekaConfig,
    pub instance: Instance,
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
        ParseError(description: String) {}
    }
}

#[derive(Debug)]
pub struct EurekaClient {
    base_url: String,
    config: BaseConfig,
    client: ReqwestClient,
    registry: RegistryClient,
    instance: Option<InstanceClient>,
}

impl EurekaClient {
    pub fn new(config: BaseConfig) -> Self {
        let base_url = {
            let ssl = config.eureka.ssl;
            let protocol = if ssl { "https" } else { "http" };
            let host = &config.eureka.host;
            let port = config.eureka.port;
            let service_path = &config.eureka.service_path;
            format!("{}://{}:{}{}", protocol, host, port, service_path)
        };
        EurekaClient {
            base_url: base_url.clone(),
            client: ReqwestClient::new(),
            registry: RegistryClient::new(base_url.clone()),
            instance: if config.eureka.register_with_eureka {
                Some(InstanceClient::new(base_url, config.instance.clone()))
            } else {
                None
            },
            config,
        }
    }

    pub fn start(&self) {
        self.registry.start();
        if let Some(ref instance) = self.instance {
            instance.start();
        }
    }

    pub fn make_request<V: Serialize>(
        &self,
        app: &str,
        path: &str,
        method: Method,
        body: &V,
    ) -> Result<Response, EurekaError> {
        let instance = self.registry.get_instance_by_app_name(app);
        if let Some(instance) = instance {
            let ssl = self.config.eureka.ssl;
            let protocol = if ssl { "https" } else { "http" };
            let host = instance.ip_addr;
            let port = if ssl && instance.secure_port.value().is_some() {
                instance.secure_port.value().unwrap()
            } else {
                instance.port.and_then(|port| port.value()).unwrap_or(8080)
            };
            self.client
                .request(
                    method,
                    &format!(
                        "{}://{}:{}/{}",
                        protocol,
                        host,
                        port,
                        path.trim_left_matches('/')
                    ),
                )
                .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
                .json(body)
                .send()
                .map_err(EurekaError::Network)
        } else {
            Err(EurekaError::UnexpectedState(format!(
                "Could not find app {}",
                app
            )))
        }
    }
}

fn path_segment_encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::PATH_SEGMENT_ENCODE_SET)
        .to_string()
}

fn query_encode(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::QUERY_ENCODE_SET).to_string()
}
