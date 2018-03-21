use EurekaError;
use rest::EurekaRestClient;
pub use rest::structures::{Instance, PortData, StatusType};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct InstanceClient {
    client: Arc<EurekaRestClient>,
    config: Arc<Instance>,
    is_running: Arc<AtomicBool>,
}

impl InstanceClient {
    pub fn new(base_url: String, config: Instance) -> Self {
        InstanceClient {
            client: Arc::new(EurekaRestClient::new(base_url)),
            config: Arc::new(config),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        while let Err(e) = self.client.register(&self.config.app, &*self.config) {
            error!("Failed to register app: {}", e);
            thread::sleep(Duration::from_secs(15));
        }
        debug!("Registered app with eureka");

        self.is_running.store(true, Ordering::Relaxed);

        let is_running = Arc::clone(&self.is_running);
        let client = Arc::clone(&self.client);
        let config = Arc::clone(&self.config);
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(30));
            while is_running.load(Ordering::Relaxed) {
                let resp = client.send_heartbeat(&config.app, &config.host_name);
                match resp {
                    Err(EurekaError::UnexpectedState(_)) => {
                        warn!("App not registered with eureka, reregistering");
                        let _ = client.register(&config.app, &*config);
                    }
                    Err(e) => {
                        error!("Failed to send heartbeat: {}", e);
                    }
                    Ok(_) => {
                        debug!("Sent heartbeat successfully");
                    }
                }
                thread::sleep(Duration::from_secs(30));
            }
        });

        while let Err(e) =
            self.client
                .update_status(&self.config.app, &self.config.host_name, &StatusType::Up)
        {
            error!("Failed to set app to UP: {}", e);
            thread::sleep(Duration::from_secs(15));
        }
    }
}

impl Drop for InstanceClient {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        let _ = self.client
            .deregister(&self.config.app, &self.config.host_name);
    }
}
