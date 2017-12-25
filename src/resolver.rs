use std::collections::HashMap;
use std::fmt::Debug;

use serde_json::Value;

pub trait ClusterResolver: Debug {
    fn resolve_eureka_url(&self, retry_attempts: usize);
}

#[derive(Debug)]
pub struct ConfigClusterResolver {}

impl ConfigClusterResolver {
    pub fn new(config: &HashMap<String, Value>) -> Self {
        unimplemented!()
    }

    fn build_service_urls(&self) {
        unimplemented!()
    }
}

impl ClusterResolver for ConfigClusterResolver {
    fn resolve_eureka_url(&self, retry_attempts: usize) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct DnsClusterResolver {}

impl DnsClusterResolver {
    pub fn new(config: &HashMap<String, Value>) -> Self {
        unimplemented!()
    }

    fn get_current_cluster(&self) {
        unimplemented!()
    }

    fn start_cluster_refresh(&self) {
        unimplemented!()
    }

    fn resolve_cluster_hosts(&self) {
        unimplemented!()
    }

    fn resolve_zone_hosts(&self) {
        unimplemented!()
    }

    fn get_availability_zones(&self) {
        unimplemented!()
    }
}

impl ClusterResolver for DnsClusterResolver {
    fn resolve_eureka_url(&self, retry_attempts: usize) {
        unimplemented!()
    }
}
