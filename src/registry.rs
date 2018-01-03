use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use itertools::Itertools;

use rest::EurekaRestClient;
use rest::structures::Instance;

#[derive(Debug)]
pub struct RegistryClient {
    client: Arc<EurekaRestClient>,
    app_cache: Arc<RwLock<HashMap<String, Vec<Instance>>>>,
    vip_cache: Arc<RwLock<HashMap<String, Vec<Instance>>>>,
    is_running: Arc<AtomicBool>,
}

impl RegistryClient {
    pub fn new(base_url: String) -> Self {
        RegistryClient {
            client: Arc::new(EurekaRestClient::new(base_url)),
            app_cache: Arc::new(RwLock::new(HashMap::new())),
            vip_cache: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        self.is_running.store(true, Ordering::Relaxed);

        let is_running = Arc::clone(&self.is_running);
        let client = Arc::clone(&self.client);
        let app_cache = Arc::clone(&self.app_cache);
        let vip_cache = Arc::clone(&self.vip_cache);
        thread::spawn(move || {
            while is_running.load(Ordering::Relaxed) {
                let resp = client.get_all_instances();
                match resp {
                    Ok(instances) => {
                        *app_cache.write().unwrap() = group_instances_by_app(instances.clone());
                        *vip_cache.write().unwrap() = group_instances_by_vip(instances);
                    }
                    Err(e) => {
                        error!("Failed to fetch registry: {}", e);
                    }
                };
                thread::sleep(Duration::from_secs(30));
            }
        });
    }
}

impl Drop for RegistryClient {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

fn group_instances_by_app(instances: Vec<Instance>) -> HashMap<String, Vec<Instance>> {
    instances
        .into_iter()
        .group_by(|i| i.app.clone())
        .into_iter()
        .map(|(k, g)| (k, g.collect()))
        .collect()
}

fn group_instances_by_vip(instances: Vec<Instance>) -> HashMap<String, Vec<Instance>> {
    instances
        .into_iter()
        .group_by(|i| i.vip_address.clone())
        .into_iter()
        .map(|(k, g)| (k, g.collect()))
        .collect()
}
