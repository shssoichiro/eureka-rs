#[derive(Debug, Clone)]
pub struct AwsMetadata {
    host: String,
}

impl AwsMetadata {
    fn fetch_metadata(&self) {
        unimplemented!()
    }

    fn lookup_metadata_key(&self) {
        unimplemented!()
    }

    fn lookup_instance_identity(&self) {
        unimplemented!()
    }
}

impl Default for AwsMetadata {
    fn default() -> Self {
        AwsMetadata {
            host: String::from("169.254.169.254"),
        }
    }
}
