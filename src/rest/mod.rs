pub mod structures;

use reqwest::{Client, Method, Response, StatusCode};

use EurekaError;
use self::structures::*;

#[derive(Debug)]
pub struct EurekaRestClient {
    client: Client,
}

impl EurekaRestClient {
    pub fn new() -> Self {
        EurekaRestClient {
            client: Client::new()
        }
    }

    /// Register new application instance
    pub fn register(app_id: &str, data: &RegisterData) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// De-register application instance
    pub fn deregister(app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// Send application instance heartbeat
    pub fn send_heartbeat(app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// Query for all instances
    pub fn get_all_instances() -> Result<Vec<Instance>, EurekaError> {
        unimplemented!()
    }

    /// Query for all `app_id` instances
    pub fn get_instances_by_app(app_id: &str) -> Result<Vec<Instance>, EurekaError> {
        unimplemented!()
    }

    /// Query for a specific `app_id/instance_id`
    pub fn get_instance_by_app_and_instance(app_id: &str, instance_id: &str) -> Result<Instance, EurekaError> {
        unimplemented!()
    }

    /// Query for a specific `instance_id`
    pub fn get_instance(instance_id: &str) -> Result<Instance, EurekaError> {
        unimplemented!()
    }

    /// Take instance out of service
    pub fn remove_from_service(app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// Put instance back into service (remove override)
    pub fn reenable_service(app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// Update metadata
    pub fn update_metadata(app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        unimplemented!()
    }

    /// Query for all instances under a particular `vip_address`
    pub fn get_instances_by_vip_address(vip_address: &str) -> Result<Vec<Instance>, EurekaError> {
        unimplemented!()
    }

    /// Query for all instances under a particular `svip_address`
    pub fn get_instances_by_svip_address(svip_address: &str) -> Result<Vec<Instance>, EurekaError> {
        unimplemented!()
    }
}
