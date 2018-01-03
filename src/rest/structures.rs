use std::collections::HashMap;
use std::fmt::{Display, Error as FmtError, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub instance: Instance,
    #[serde(rename = "dataCenterInfo")] pub data_center_info: DataCenterInfo,
    #[serde(rename = "leaseInfo")] pub lease_info: LeaseInfo,
    #[serde(rename = "dcNameType")] pub dc_name_type: DcNameType,
    #[serde(rename = "statusType")] pub status_type: StatusType,
    // This is a typo on Eureka's side
    #[serde(rename = "amazonMetdataType")] pub amazon_metadata_type: Option<AmazonMetadataType>,
    #[serde(rename = "appMetadataType")] pub app_metadata_type: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    #[serde(rename = "hostName")] pub host_name: String,
    pub app: String,
    #[serde(rename = "ipAddr")] pub ip_addr: String,
    #[serde(rename = "vipAddress")] pub vip_address: String,
    #[serde(rename = "secureVipAddress")] pub secure_vip_address: String,
    pub status: StatusType,
    pub port: Option<u16>,
    #[serde(rename = "securePort")] pub secure_port: u16,
    #[serde(rename = "homePageUrl")] pub home_page_url: String,
    #[serde(rename = "statusPageUrl")] pub status_page_url: String,
    #[serde(rename = "healthCheckUrl")] pub health_check_url: String,
    #[serde(rename = "dataCenterInfo")] pub data_center_info: DataCenterInfo,
    #[serde(rename = "leaseInfo")] pub lease_info: Option<LeaseInfo>,
    /// optional app specific metadata
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCenterInfo {
    pub name: DcNameType,
    /// metadata is only required if name is Amazon
    pub metadata: Option<AmazonMetadataType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    /// (optional) if you want to change the length of lease - default if 90 secs
    #[serde(rename = "evictionDurationInSecs")]
    pub eviction_duration_in_secs: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DcNameType {
    MyOwn,
    Amazon,
}

impl Display for DcNameType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
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

impl Display for StatusType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmazonMetadataType {
    #[serde(rename = "ami-launch-index")] pub ami_launch_index: String,
    #[serde(rename = "local-hostname")] pub local_hostname: String,
    #[serde(rename = "availability-zone")] pub availability_zone: String,
    #[serde(rename = "instance-id")] pub instance_id: String,
    #[serde(rename = "public-ipv4")] pub public_ipv4: String,
    #[serde(rename = "public-hostname")] pub public_hostname: String,
    #[serde(rename = "ami-manifest-path")] pub ami_manifest_path: String,
    #[serde(rename = "local-ipv4")] pub local_ipv4: String,
    #[serde(rename = "hostname")] pub hostname: String,
    #[serde(rename = "ami-id")] pub ami_id: String,
    #[serde(rename = "instance-type")] pub instance_type: String,
}
