use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {}

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

impl RegisterData {
    pub fn new(
        host_name: String,
        app_name: String,
        ip_addr: String,
        vip_addr: String,
        secure_vip_addr: String,
        port: Option<u16>,
        secure_port: Option<u16>,
        home_page_url: String,
        status_page_url: String,
        health_check_url: String,
        dc_name_type: DcNameType,
        dc_metadata: Option<AmazonMetadataType>,
        amazon_metadata: Option<AmazonMetadataType>,
        metadata: HashMap<String, String>,
    ) -> Self {
        RegisterData {
            instance: Instance {
                host_name,
                app: app_name,
                ip_addr,
                vip_address: vip_addr,
                secure_vip_address: secure_vip_addr,
                status: StatusType::UP,
                port: PortInfo::new(port, false),
                secure_port: PortInfo::new(secure_port, true),
                home_page_url,
                status_page_url,
                health_check_url,
                data_center_info: DataCenterInfo {
                    name: dc_name_type.clone(),
                    metadata: dc_metadata.clone(),
                },
                lease_info: LeaseInfo {
                    eviction_duration_in_secs: None,
                },
                metadata: metadata.clone(),
            },
            data_center_info: DataCenterInfo {
                name: dc_name_type.clone(),
                metadata: dc_metadata,
            },
            lease_info: LeaseInfo {
                eviction_duration_in_secs: None,
            },
            dc_name_type,
            status_type: StatusType::UP,
            amazon_metadata_type: amazon_metadata,
            app_metadata_type: metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    #[serde(rename = "hostName")] pub host_name: String,
    pub app: String,
    #[serde(rename = "ipAddr")] pub ip_addr: String,
    #[serde(rename = "vipAddress")] pub vip_address: String,
    #[serde(rename = "secureVipAddress")] pub secure_vip_address: String,
    pub status: StatusType,
    pub port: PortInfo,
    #[serde(rename = "securePort")] pub secure_port: PortInfo,
    #[serde(rename = "homePageUrl")] pub home_page_url: String,
    #[serde(rename = "statusPageUrl")] pub status_page_url: String,
    #[serde(rename = "healthCheckUrl")] pub health_check_url: String,
    #[serde(rename = "dataCenterInfo")] pub data_center_info: DataCenterInfo,
    #[serde(rename = "leaseInfo")] pub lease_info: LeaseInfo,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    #[serde(rename = "$")] pub value: u16,
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
    #[serde(rename = "evictionDurationInSecs")] pub eviction_duration_in_secs: Option<usize>,
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
