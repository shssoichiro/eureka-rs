use std::collections::HashMap;

use reqwest::{Client, Response};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AwsMetadata {
    client: Client,
    host: String,
}

impl AwsMetadata {
    pub fn new(config: &HashMap<String, Value>) -> Self {
        AwsMetadata {
            client: Client::new(),
            host: config
                .get("host")
                .map(|host| host.as_str().unwrap().to_string())
                .unwrap_or_else(|| String::from("169.254.169.254")),
        }
    }

    fn fetch_metadata(&self) -> HashMap<&'static str, String> {
        let mut results = HashMap::with_capacity(11);
        results.insert("ami-id", self.lookup_metadata_key("ami-id"));
        results.insert("instance-id", self.lookup_metadata_key("instance-id"));
        results.insert("instance-type", self.lookup_metadata_key("instance-type"));
        results.insert("local-ipv4", self.lookup_metadata_key("local-ipv4"));
        results.insert("local-hostname", self.lookup_metadata_key("local-hostname"));
        results.insert(
            "availability-zone",
            self.lookup_metadata_key("placement/availability-zone"),
        );
        results.insert(
            "public-hostname",
            self.lookup_metadata_key("public-hostname"),
        );
        results.insert("public-ipv4", self.lookup_metadata_key("public-ipv4"));
        results.insert("mac", self.lookup_metadata_key("mac"));
        results.insert(
            "accountId",
            self.lookup_instance_identity()
                .and_then(|i| i["accountId"].as_str().map(|id| id.to_owned())),
        );
        let mac = results["mac"].clone().unwrap();
        results.insert(
            "vpc-id",
            self.lookup_metadata_key(&format!("network/interfaces/macs/{}/vpc-id", mac)),
        );
        debug!("Found Instance AWS Metadata: {:?}", results);
        results
            .into_iter()
            .fold(HashMap::new(), |mut filtered, (prop, value)| {
                if let Some(value) = value {
                    filtered.insert(prop, value);
                }
                filtered
            })
    }

    fn lookup_metadata_key(&self, key: &str) -> Option<String> {
        let mut response = self.client
            .get(&format!("http://{}/latest/meta-data/{}", self.host, key))
            .send()
            .and_then(Response::error_for_status)
            .map_err(|e| {
                error!("Error requesting metadata key: {}", e);
                e
            })
            .ok()?;
        response.text().ok()
    }

    fn lookup_instance_identity(&self) -> Option<HashMap<String, Value>> {
        let mut response = self.client
            .get(&format!(
                "http://{}/latest/dynamic/instance-identity/document",
                self.host
            ))
            .send()
            .and_then(Response::error_for_status)
            .map_err(|e| {
                error!("Error requesting instance identity document: {}", e);
                e
            })
            .ok()?;
        response.json().ok()
    }
}
