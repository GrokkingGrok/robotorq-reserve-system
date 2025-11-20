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
        let mut config: Config = serde_yaml::from_str(&contents)?;
        
        // Override with environment variables if present
        config.apply_env_overrides();
        
        Ok(config)
    }
    
    pub fn from_env() -> Self {
        let mut config = Config::default();
        config.apply_env_overrides();
        config
    }
    
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("PRINTER_ID") {
            self.printer_id = val;
        }
        if let Ok(val) = std::env::var("PRINTER_MODEL") {
            self.printer_model = val;
        }
        if let Ok(val) = std::env::var("RATED_WATTS") {
            if let Ok(watts) = val.parse() {
                self.rated_watts = watts;
            }
        }
        if let Ok(val) = std::env::var("NATS_URL") {
            self.nats_url = val;
        }
        if let Ok(val) = std::env::var("KLIPPER_URL") {
            self.klipper_url = val;
        }
        if let Ok(val) = std::env::var("MOCK_MODE") {
            if let Ok(mode) = val.parse() {
                self.mock_mode = mode;
            }
        }
        if let Ok(val) = std::env::var("LOG_LEVEL") {
            self.log_level = val;
        }
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
