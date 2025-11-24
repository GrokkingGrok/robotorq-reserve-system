use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintConfig {
    pub nats_url: String,
    pub ingots_per_cert: usize,
    // Add more config fields as needed
}

impl MintConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            ingots_per_cert: env::var("INGOTS_PER_CERT")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
        })
    }
}