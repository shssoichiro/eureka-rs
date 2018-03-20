use std::collections::HashMap;
use std::fmt::{Display, Error as FmtError, Formatter};

use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterData {
    pub instance: Instance,
    pub data_center_info: DataCenterInfo,
    pub lease_info: Option<LeaseInfo>,
    pub dc_name_type: DcNameType,
    pub status_type: StatusType,
    // This is a typo on Eureka's side
    #[serde(rename = "amazonMetdataType")]
    pub amazon_metadata_type: Option<AmazonMetadataType>,
    pub app_metadata_type: HashMap<String, String>,
}

impl From<Instance> for RegisterData {
    fn from(instance: Instance) -> Self {
        RegisterData {
            data_center_info: instance.data_center_info.clone(),
            lease_info: instance.lease_info,
            dc_name_type: instance.data_center_info.name,
            status_type: StatusType::Starting,
            amazon_metadata_type: None,
            app_metadata_type: HashMap::new(),
            instance,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// This doubles as the instance ID, because why not Eureka?
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    #[serde(rename = "$")]
    value: u16,
    #[serde(rename = "@enabled")]
    enabled: String,
}

impl PortData {
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
    pub name: DcNameType,
    /// metadata is only required if name is Amazon
    pub metadata: Option<AmazonMetadataType>,
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
