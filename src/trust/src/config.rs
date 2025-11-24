use anyhow::{anyhow, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct TrustConfig {
    pub nats_url: String,
    pub genesis_contract_path: String,
    pub metrics_host: String,
    pub metrics_port: u16,
    pub owner_name: Option<String>,
    pub owner_wallet_id: Option<String>,
}

impl TrustConfig {
    pub fn from_env() -> Result<Self> {
        let nats_url = env::var("TRUST_NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
        let genesis_contract_path = env::var("TRUST_GENESIS_CONTRACT_PATH").unwrap_or_else(|_| "config/genesis_contract.json".to_string());
        let metrics_host = env::var("TRUST_METRICS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let metrics_port = env::var("TRUST_METRICS_PORT").unwrap_or_else(|_| "8082".to_string()).parse::<u16>().map_err(|e| anyhow!(e.to_string()))?;
        let owner_name = env::var("TRUST_OWNER_NAME").ok();
        let owner_wallet_id = env::var("TRUST_OWNER_WALLET_ID").ok();

        Ok(TrustConfig { nats_url, genesis_contract_path, metrics_host, metrics_port, owner_name, owner_wallet_id })
    }
}

impl Default for TrustConfig {
    fn default() -> Self {
        TrustConfig {
            nats_url: "nats://nats:4222".to_string(),
            genesis_contract_path: "config/genesis_contract.json".to_string(),
            metrics_host: "0.0.0.0".to_string(),
            metrics_port: 8082,
            owner_name: Some("Jonathan Joseph Clark".to_string()),
            owner_wallet_id: None,
        }
    }
}
