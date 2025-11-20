use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub printer_id: String,
    pub printer_model: String,
    pub rated_watts: u32,
    pub nats_url: String,
    pub klipper_url: String,
    
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    
    #[serde(default = "default_log_level")]
    pub log_level: String,
    
    #[serde(default = "default_mock_mode")]
    pub mock_mode: bool,
}

fn default_heartbeat_interval() -> u64 {
    10
}

fn default_metrics_port() -> u16 {
    9091
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_mock_mode() -> bool {
    false
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            printer_id: "ender3-v3-ke-001".to_string(),
            printer_model: "Ender3V3KE".to_string(),
            rated_watts: 350,
            nats_url: "nats://localhost:4222".to_string(),
            klipper_url: "http://localhost:7125".to_string(),
            heartbeat_interval_secs: 10,
            metrics_port: 9091,
            log_level: "info".to_string(),
            mock_mode: false,
        }
    }
}
