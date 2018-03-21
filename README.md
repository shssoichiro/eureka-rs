# Eureka Client for Rust

This project is currently in an **alpha state** as indicated by the version number.

### What Works

- Registering a service with Eureka
- Sending keep-alive heartbeats to Eureka

### What is implemented but untested

- Making requests to services connected via Eureka

### What is not implemented

- DNS and AWS resolvers

## Installation

Add `eureka` to your `Cargo.toml` and add `extern crate eureka` to your project's root.

## Usage

To initialize a eureka client and register with eureka, you'll do something similar to this:

```rust
use eureka::{BaseConfig, EurekaClient, PortData};

pub fn init_eureka(
    server_host: String,
    server_port: u16,
    instance_ip_addr: String,
    instance_port: u16,
) -> EurekaClient {
    let mut config = BaseConfig::default();
    config.eureka.host = server_host;
    config.eureka.port = server_port;
    config.instance.ip_addr = instance_ip_addr;
    config.instance.port = Some(PortData::new(instance_port, true));
    let eureka = EurekaClient::new(config);
    eureka.start();
    eureka
}
```

You'll need to keep this client alive for as long as you intend to be connected to Eureka.
For example, in Rocket, you can manage it as state and access it via your routes as you would with other state, e.g. calling our above function:

```rust
rocket::ignite()
    .mount("/api/", routes![])
    .manage(init_eureka(
        dotenv!("EUREKA_HOST").into(),
        dotenv!("EUREKA_PORT")
            .parse()
            .expect("Eureka port not valid"),
        env::var("EUREKA_INSTANCE_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
        env::var("ROCKET_PORT")
            .map_err(|_| ())
            .and_then(|port| port.parse().map_err(|_| ()))
            .unwrap_or(8080),
    ))
```

This client registers with eureka by default. You can disable registration by setting `config.eureka.register_with_eureka = false`
if you just want to use this client to make requests.
