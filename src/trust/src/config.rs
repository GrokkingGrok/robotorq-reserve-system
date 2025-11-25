use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustConfig {
    // Connection
    pub nats_url: String,

    // Genesis / contract store
    pub genesis_contract_path: String,

    // HTTP / metrics
    pub metrics_host: String,
    pub metrics_port: u16,
    pub http_host: String,
    pub http_port: u16,

    // Admin
    pub admin_nats_subject: String,
    pub allow_plugin_registration: bool,

    // Operational
    pub shutdown_grace_seconds: u64,
    pub log_level: String,
    pub max_request_bytes: usize,

    // Owner metadata (optional)
    pub owner_name: Option<String>,
    pub owner_wallet_id: Option<String>,

    // Simulation fields (optional build)
    #[cfg(feature = "simulation")]
    pub simulation_mode: bool,
    #[cfg(feature = "simulation")]
    pub time_compression_factor: f64,
}

impl TrustConfig {
    pub fn from_env() -> Result<Self> {
        let nats_url = env::var("TRUST_NATS_URL")
            .or_else(|_| env::var("NATS_URL"))
            .unwrap_or_else(|_| "nats://nats:4222".to_string());

        let genesis_contract_path = env::var("TRUST_GENESIS_CONTRACT_PATH")
            .unwrap_or_else(|_| "config/genesis_contract.json".to_string());

        let metrics_host = env::var("TRUST_METRICS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let metrics_port = env::var("TRUST_METRICS_PORT").unwrap_or_else(|_| "9092".to_string())
            .parse::<u16>()
            .map_err(|e| anyhow!(e.to_string()))?;

        let http_host = env::var("TRUST_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let http_port = env::var("TRUST_HTTP_PORT").unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|e| anyhow!(e.to_string()))?;

        let admin_nats_subject = env::var("TRUST_ADMIN_NATS_SUBJECT").unwrap_or_else(|_| "trust.admin".to_string());
        let allow_plugin_registration = env::var("TRUST_ALLOW_PLUGIN_REGISTRATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let shutdown_grace_seconds = env::var("TRUST_SHUTDOWN_GRACE_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5u64);

        let log_level = env::var("TRUST_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let max_request_bytes = env::var("TRUST_MAX_REQUEST_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024usize);

        let owner_name = env::var("TRUST_OWNER_NAME").ok();
        let owner_wallet_id = env::var("TRUST_OWNER_WALLET_ID").ok();

        #[cfg(feature = "simulation")]
        let (simulation_mode, time_compression_factor) = {
            let simulation_mode = env::var("TRUST_SIMULATION_MODE").ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false);
            let time_compression_factor = env::var("TRUST_TIME_COMPRESSION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0);
            (simulation_mode, time_compression_factor)
        };

        Ok(Self {
            nats_url,
            genesis_contract_path,
            metrics_host,
            metrics_port,
            http_host,
            http_port,
            admin_nats_subject,
            allow_plugin_registration,
            shutdown_grace_seconds,
            log_level,
            max_request_bytes,
            owner_name,
            owner_wallet_id,
            #[cfg(feature = "simulation")]
            simulation_mode,
            #[cfg(feature = "simulation")]
            time_compression_factor,
        })
    }
}

impl Default for TrustConfig {
    fn default() -> Self {
        TrustConfig {
            nats_url: "nats://nats:4222".to_string(),
            genesis_contract_path: "config/genesis_contract.json".to_string(),
            metrics_host: "0.0.0.0".to_string(),
            metrics_port: 9092,
            http_host: "0.0.0.0".to_string(),
            http_port: 8080,
            admin_nats_subject: "trust.admin".to_string(),
            allow_plugin_registration: false,
            shutdown_grace_seconds: 5,
            log_level: "info".to_string(),
            max_request_bytes: 1024,
            owner_name: Some("Jonathan Joseph Clark".to_string()),
            owner_wallet_id: None,
            #[cfg(feature = "simulation")]
            simulation_mode: false,
            #[cfg(feature = "simulation")]
            time_compression_factor: 1.0,
        }
    }
}
