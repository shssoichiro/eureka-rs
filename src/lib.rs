#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod register;

use std::cmp;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use hyper::header::{Accept, ContentType, qitem};
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use hyper::status::StatusCode;
use reqwest::Client as ReqwestClient;
use reqwest::Error as ReqwestError;

use register::RegisterData;

lazy_static! {
    static ref REQWEST_CLIENT: ReqwestClient = ReqwestClient::new().unwrap();
}

#[derive(Debug)]
pub struct EurekaClient {
    server_base: String,
    client: ReqwestClient,
    app_id: String,
    instance_id: String,
}

impl EurekaClient {
    pub fn connect(server_url: &str,
                   app_id: String,
                   data: &RegisterData)
                   -> Result<(), EurekaError> {
        let mut client = EurekaClient {
            server_base: server_url.trim_right_matches('/').to_string(),
            client: ReqwestClient::new().unwrap(),
            app_id: app_id,
            instance_id: String::new(),
        };
        client.instance_id = client.register(data)?;
        info!("Connected {} on {}:{} to eureka server at {}",
              data.instance.app,
              data.instance.ip_addr,
              data.instance.port.value,
              server_url);
        let eviction_duration = data.lease_info.eviction_duration_in_secs.unwrap_or(90) as u64;
        thread::spawn(move || {
            let sleep_duration = Duration::new(cmp::max(1, eviction_duration / 2), 0);
            loop {
                thread::sleep(sleep_duration);
                if client.send_heartbeat().is_err() {
                    client.instance_id = String::new();
                    warn!("Lost connection to eureka server!");
                    break;
                };
            }
        });
        Ok(())
    }

    fn register(&self, data: &RegisterData) -> Result<String, EurekaError> {
        let resp = REQWEST_CLIENT.post(&format!("{}/v2/apps/{}", self.server_base, self.app_id))
            .header(Accept(vec![qitem(Mime(TopLevel::Application,
                                           SubLevel::Json,
                                           vec![(Attr::Charset, Value::Utf8)]))]))
            .header(ContentType(Mime(TopLevel::Application,
                                     SubLevel::Json,
                                     vec![(Attr::Charset, Value::Utf8)])))
            .body(serde_json::to_string(&data).unwrap())
            .send()
            .map_err(EurekaError::Network)?;
        if resp.status().is_success() {
            Ok(self.get_current_instance_id(data)?)
        } else {
            Err(EurekaError::Request(*resp.status()))
        }
    }

    fn get_current_instance_id(&self, data: &RegisterData) -> Result<String, EurekaError> {
        let instances = self.get_app_instances(&self.app_id)?;
        if let Some(instance) = instances
               .iter()
               .find(|i| {
            i.host_name == data.instance.host_name && i.app == data.instance.app &&
            i.ip_addr == data.instance.ip_addr &&
            i.vip_address == data.instance.vip_address &&
            i.secure_vip_address == data.instance.secure_vip_address &&
            i.status == data.instance.status && i.port == data.instance.port &&
            i.secure_port == data.instance.secure_port
        }) {
            Ok(instance.id.clone())
        } else {
            Err(EurekaError::UnexpectedState("Newly created instance not found"))
        }
    }

    fn deregister(&self) -> Result<(), EurekaError> {
        let resp = REQWEST_CLIENT.delete(&format!("{}/v2/apps/{}/{}",
                             self.server_base,
                             self.app_id,
                             self.instance_id))
            .header(Accept(vec![qitem(Mime(TopLevel::Application,
                                           SubLevel::Json,
                                           vec![(Attr::Charset, Value::Utf8)]))]))
            .send()
            .map_err(EurekaError::Network)?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(EurekaError::Request(*resp.status()))
        }
    }

    fn send_heartbeat(&self) -> Result<(), EurekaError> {
        let resp = REQWEST_CLIENT.put(&format!("{}/v2/apps/{}/{}",
                          self.server_base,
                          self.app_id,
                          self.instance_id))
            .header(Accept(vec![qitem(Mime(TopLevel::Application,
                                           SubLevel::Json,
                                           vec![(Attr::Charset, Value::Utf8)]))]))
            .send()
            .map_err(EurekaError::Network)?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(EurekaError::Request(*resp.status()))
        }
    }

    fn get_app_instances(&self, app_id: &str) -> Result<Vec<AppInstance>, EurekaError> {
        let mut resp = REQWEST_CLIENT.get(&format!("{}/v2/apps/{}", self.server_base, app_id))
            .header(Accept(vec![qitem(Mime(TopLevel::Application,
                                           SubLevel::Json,
                                           vec![(Attr::Charset, Value::Utf8)]))]))
            .send()
            .map_err(EurekaError::Network)?;
        if resp.status().is_success() {
            Ok(resp.json().unwrap())
        } else {
            Err(EurekaError::Request(*resp.status()))
        }
    }
}

impl Drop for EurekaClient {
    fn drop(&mut self) {
        if !self.instance_id.is_empty() {
            let _ = self.deregister();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub id: String,
    #[serde(rename = "hostName")]
    pub host_name: String,
    pub app: String,
    #[serde(rename = "ipAddr")]
    pub ip_addr: String,
    #[serde(rename = "vipAddress")]
    pub vip_address: String,
    #[serde(rename = "secureVipAddress")]
    pub secure_vip_address: String,
    pub status: StatusType,
    pub port: PortInfo,
    #[serde(rename = "securePort")]
    pub secure_port: PortInfo,
    #[serde(rename = "homePageUrl")]
    pub home_page_url: String,
    #[serde(rename = "statusPageUrl")]
    pub status_page_url: String,
    #[serde(rename = "healthCheckUrl")]
    pub health_check_url: String,
    #[serde(rename = "dataCenterInfo")]
    pub data_center_info: DataCenterInfo,
    #[serde(rename = "leaseInfo")]
    pub lease_info: LeaseInfo,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortInfo {
    #[serde(rename = "$")]
    pub value: u16,
    pub enabled: bool,
}

impl PortInfo {
    pub fn new(port: Option<u16>, secure: bool) -> Self {
        PortInfo {
            value: port.unwrap_or_else(|| if secure { 443 } else { 80 }),
            enabled: port.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCenterInfo {
    pub name: DcNameType,
    /// metadata is only required if name is Amazon
    pub metadata: Option<AmazonMetadataType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    #[serde(rename = "evictionDurationInSecs")]
    pub eviction_duration_in_secs: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DcNameType {
    MyOwn,
    Amazon,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StatusType {
    UP,
    DOWN,
    STARTING,
    OUT_OF_SERVICE,
    UNKNOWN,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmazonMetadataType {
    #[serde(rename = "ami-launch-index")]
    pub ami_launch_index: String,
    #[serde(rename = "local-hostname")]
    pub local_hostname: String,
    #[serde(rename = "availability-zone")]
    pub availability_zone: String,
    #[serde(rename = "instance-id")]
    pub instance_id: String,
    #[serde(rename = "public-ipv4")]
    pub public_ipv4: String,
    #[serde(rename = "public-hostname")]
    pub public_hostname: String,
    #[serde(rename = "ami-manifest-path")]
    pub ami_manifest_path: String,
    #[serde(rename = "local-ipv4")]
    pub local_ipv4: String,
    #[serde(rename = "hostname")]
    pub hostname: String,
    #[serde(rename = "ami-id")]
    pub ami_id: String,
    #[serde(rename = "instance-type")]
    pub instance_type: String,
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
        UnexpectedState(description: &'static str) {
            description(description)
        }
    }
}
