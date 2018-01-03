use std::sync::Arc;

use rest::EurekaRestClient;
use rest::structures::Instance;

#[derive(Debug)]
pub struct InstanceClient {
    client: Arc<EurekaRestClient>,
    config: Instance,
}

impl InstanceClient {
    pub fn new(base_url: String, config: Instance) -> Self {
        InstanceClient {
            client: Arc::new(EurekaRestClient::new(base_url)),
            config,
        }
    }

    pub fn start(&self) {
        unimplemented!()
    }
}
