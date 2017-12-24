use std::collections::HashMap;

use reqwest::Response;
use serde_json::Value;

use REQWEST_CLIENT;

#[derive(Debug, Clone)]
pub struct AwsMetadata {
    host: String,
}

impl AwsMetadata {
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
        let mut response = REQWEST_CLIENT
            .get(&format!("http://{}/latest/meta-data/{}", self.host, key))
            .send()
            .map_err(|e| {
                error!("Error requesting metadata key: {}", e);
                e
            })
            .and_then(Response::error_for_status)
            .ok()?;
        response.text().ok()
    }

    fn lookup_instance_identity(&self) -> Option<HashMap<String, Value>> {
        let mut response = REQWEST_CLIENT
            .get(&format!(
                "http://{}/latest/dynamic/instance-identity/document",
                self.host
            ))
            .send()
            .map_err(|e| {
                error!("Error requesting instance identity document: {}", e);
                e
            })
            .and_then(Response::error_for_status)
            .ok()?;
        response.json().ok()
    }
}

impl Default for AwsMetadata {
    fn default() -> Self {
        AwsMetadata {
            host: String::from("169.254.169.254"),
        }
    }
}
