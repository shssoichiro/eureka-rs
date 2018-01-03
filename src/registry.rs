use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use itertools::Itertools;

use rest::EurekaRestClient;
use rest::structures::Instance;

#[derive(Debug)]
pub struct RegistryClient {
    client: Arc<EurekaRestClient>,
    cache: Arc<Mutex<HashMap<String, Vec<Instance>>>>,
    is_running: Arc<AtomicBool>,
}

impl RegistryClient {
    pub fn new(base_url: String) -> Self {
        RegistryClient {
            client: Arc::new(EurekaRestClient::new(base_url)),
            cache: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) {
        self.is_running.store(true, Ordering::Relaxed);
        let is_running = Arc::clone(&self.is_running);
        let client = Arc::clone(&self.client);
        let cache = Arc::clone(&self.cache);
        thread::spawn(move || {
            while is_running.load(Ordering::Relaxed) {
                let resp = client.get_all_instances();
                match resp {
                    Ok(instances) => {
                        *cache.lock().unwrap() = group_instances(instances);
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

fn group_instances(instances: Vec<Instance>) -> HashMap<String, Vec<Instance>> {
    instances
        .into_iter()
        .group_by(|i| i.app.clone())
        .into_iter()
        .map(|(k, g)| (k, g.collect()))
        .collect()
}
