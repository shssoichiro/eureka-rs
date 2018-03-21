use serde_json;
use std::collections::HashMap;
use std::env;
use std::fmt::{Display, Error as FmtError, Formatter};

#[derive(Debug, Clone, Serialize)]
pub struct Register<'a> {
    pub instance: &'a Instance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// This doubles as the instance ID, because why not, Eureka?
    pub host_name: String,
    pub app: String,
    pub ip_addr: String,
    pub vip_address: String,
    pub secure_vip_address: String,
    pub status: StatusType,
    pub port: Option<PortData>,
    pub secure_port: PortData,
    pub home_page_url: String,
    pub status_page_url: String,
    pub health_check_url: String,
    pub data_center_info: DataCenterInfo,
    pub lease_info: Option<LeaseInfo>,
    /// optional app specific metadata
    pub metadata: Option<HashMap<String, String>>,
}

impl Default for Instance {
    fn default() -> Self {
        Instance {
            host_name: "localhost".to_string(),
            app: env::var("CARGO_PKG_NAME").unwrap_or_default(),
            ip_addr: "127.0.0.1".to_string(),
            vip_address: env::var("CARGO_PKG_NAME").unwrap_or_default(),
            secure_vip_address: env::var("CARGO_PKG_NAME").unwrap_or_default(),
            status: StatusType::Starting,
            port: None,
            secure_port: PortData::new(443, false),
            home_page_url: String::new(),
            status_page_url: String::new(),
            health_check_url: String::new(),
            data_center_info: DataCenterInfo::default(),
            lease_info: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    #[serde(rename = "$")]
    value: u16,
    #[serde(rename = "@enabled")]
    enabled: String,
}

impl PortData {
    pub fn new(port: u16, enabled: bool) -> Self {
        PortData {
            value: port,
            enabled: enabled.to_string(),
        }
    }

    pub fn value(&self) -> Option<u16> {
        if self.enabled == "true" {
            Some(self.value)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AllApplications {
    pub applications: Applications,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Applications {
    pub application: Vec<Application>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationWrapper {
    pub application: Application,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Application {
    pub instance: Vec<Instance>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstanceWrapper {
    pub instance: Instance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCenterInfo {
    #[serde(rename = "@class")]
    class: String,
    pub name: DcNameType,
    /// metadata is only allowed if name is Amazon, and then is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AmazonMetadataType>,
}

impl Default for DataCenterInfo {
    fn default() -> Self {
        DataCenterInfo {
            class: "com.netflix.appinfo.InstanceInfo$DefaultDataCenterInfo".into(),
            name: DcNameType::MyOwn,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseInfo {
    /// (optional) if you want to change the length of lease - default if 90 secs
    pub eviction_duration_in_secs: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DcNameType {
    MyOwn,
    Amazon,
}

impl Display for DcNameType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusType {
    Up,
    Down,
    Starting,
    OutOfService,
    Unknown,
}

impl Display for StatusType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(
            f,
            "{}",
            serde_json::to_value(self).unwrap().as_str().unwrap()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmazonMetadataType {
    pub ami_launch_index: String,
    pub local_hostname: String,
    pub availability_zone: String,
    pub instance_id: String,
    pub public_ipv4: String,
    pub public_hostname: String,
    pub ami_manifest_path: String,
    pub local_ipv4: String,
    pub hostname: String,
    pub ami_id: String,
    pub instance_type: String,
}
