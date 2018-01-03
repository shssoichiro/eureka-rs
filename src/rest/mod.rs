pub mod structures;

use reqwest::{Client, StatusCode};

use {path_segment_encode, query_encode, EurekaError};
use self::structures::*;

#[derive(Debug)]
pub struct EurekaRestClient {
    client: Client,
    base_url: String,
}

impl EurekaRestClient {
    pub fn new(base_url: String) -> Self {
        EurekaRestClient {
            client: Client::new(),
            base_url,
        }
    }

    /// Register new application instance
    pub fn register(&self, app_id: &str, data: &RegisterData) -> Result<(), EurekaError> {
        let resp = self.client
            .post(&format!(
                "{}/eureka/apps/{}",
                self.base_url,
                path_segment_encode(app_id)
            ))
            .json(data)
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(resp) => match resp.status() {
                StatusCode::NoContent => Ok(()),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// De-register application instance
    pub fn deregister(&self, app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        let resp = self.client
            .delete(&format!(
                "{}/eureka/apps/{}/{}",
                self.base_url,
                path_segment_encode(app_id),
                path_segment_encode(instance_id)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(resp) => match resp.status() {
                StatusCode::Ok => Ok(()),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Send application instance heartbeat
    pub fn send_heartbeat(&self, app_id: &str, instance_id: &str) -> Result<(), EurekaError> {
        let resp = self.client
            .delete(&format!(
                "{}/eureka/apps/{}/{}",
                self.base_url,
                path_segment_encode(app_id),
                path_segment_encode(instance_id)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(resp) => match resp.status() {
                StatusCode::Ok => Ok(()),
                StatusCode::NotFound => Err(EurekaError::UnexpectedState(
                    "Instance does not exist".into(),
                )),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for all instances
    pub fn get_all_instances(&self) -> Result<Vec<Instance>, EurekaError> {
        let resp = self.client
            .get(&format!("{}/eureka/apps", self.base_url))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for all `app_id` instances
    pub fn get_instances_by_app(&self, app_id: &str) -> Result<Vec<Instance>, EurekaError> {
        let resp = self.client
            .get(&format!(
                "{}/eureka/apps/{}",
                self.base_url,
                path_segment_encode(app_id)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for a specific `app_id/instance_id`
    pub fn get_instance_by_app_and_instance(
        &self,
        app_id: &str,
        instance_id: &str,
    ) -> Result<Instance, EurekaError> {
        let resp = self.client
            .get(&format!(
                "{}/eureka/apps/{}/{}",
                self.base_url,
                path_segment_encode(app_id),
                path_segment_encode(instance_id)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for a specific `instance_id`
    pub fn get_instance(&self, instance_id: &str) -> Result<Instance, EurekaError> {
        let resp = self.client
            .get(&format!(
                "{}/eureka/apps/{}",
                self.base_url,
                path_segment_encode(instance_id)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Update instance status
    pub fn update_status(
        &self,
        app_id: &str,
        instance_id: &str,
        new_status: &StatusType,
    ) -> Result<(), EurekaError> {
        let resp = self.client
            .put(&format!(
                "{}/eureka/apps/{}/{}/status?value={}",
                self.base_url,
                path_segment_encode(app_id),
                path_segment_encode(instance_id),
                new_status
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(resp) => match resp.status() {
                StatusCode::Ok => Ok(()),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Update metadata
    pub fn update_metadata(
        &self,
        app_id: &str,
        instance_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), EurekaError> {
        let resp = self.client
            .put(&format!(
                "{}/eureka/apps/{}/{}/metadata?{}={}",
                self.base_url,
                path_segment_encode(app_id),
                path_segment_encode(instance_id),
                query_encode(key),
                query_encode(value)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(resp) => match resp.status() {
                StatusCode::Ok => Ok(()),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for all instances under a particular `vip_address`
    pub fn get_instances_by_vip_address(
        &self,
        vip_address: &str,
    ) -> Result<Vec<Instance>, EurekaError> {
        let resp = self.client
            .get(&format!(
                "{}/eureka/vips/{}",
                self.base_url,
                path_segment_encode(vip_address)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }

    /// Query for all instances under a particular `svip_address`
    pub fn get_instances_by_svip_address(
        &self,
        svip_address: &str,
    ) -> Result<Vec<Instance>, EurekaError> {
        let resp = self.client
            .get(&format!(
                "{}/eureka/svips/{}",
                self.base_url,
                path_segment_encode(svip_address)
            ))
            .send();
        match resp {
            Err(e) => Err(EurekaError::Network(e)),
            Ok(mut resp) => match resp.status() {
                StatusCode::Ok => Ok(resp.json()
                    .map_err(|e| EurekaError::ParseError(e.to_string()))?),
                _ => Err(EurekaError::Request(resp.status())),
            },
        }
    }
}
