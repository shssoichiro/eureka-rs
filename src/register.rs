use std::collections::HashMap;

use super::{DataCenterInfo, LeaseInfo, DcNameType, StatusType, AmazonMetadataType, PortInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub instance: Instance,
    #[serde(rename = "dataCenterInfo")]
    pub data_center_info: DataCenterInfo,
    #[serde(rename = "leaseInfo")]
    pub lease_info: LeaseInfo,
    #[serde(rename = "dcNameType")]
    pub dc_name_type: DcNameType,
    #[serde(rename = "statusType")]
    pub status_type: StatusType,
    // This is a typo on Eureka's side
    #[serde(rename = "amazonMetdataType")]
    pub amazon_metadata_type: Option<AmazonMetadataType>,
    #[serde(rename = "appMetadataType")]
    pub app_metadata_type: HashMap<String, String>,
}

impl RegisterData {
    #[cfg_attr(feature="clippy", allow(too_many_arguments))]
    pub fn new(host_name: String,
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
               amazon_metadata: Option<AmazonMetadataType>,
               metadata: HashMap<String, String>)
               -> Self {
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
                    metadata: amazon_metadata.clone(),
                },
                lease_info: LeaseInfo { eviction_duration_in_secs: None },
                metadata: metadata.clone(),
            },
            data_center_info: DataCenterInfo {
                name: dc_name_type.clone(),
                metadata: amazon_metadata.clone(),
            },
            lease_info: LeaseInfo { eviction_duration_in_secs: None },
            dc_name_type,
            status_type: StatusType::UP,
            amazon_metadata_type: amazon_metadata,
            app_metadata_type: metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
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
