use anyhow::Result;
use std::sync::Arc;

mod config;
mod handlers;
mod http;
mod models;
mod store;

use config::TrustConfig;
use handlers::nats_handler::NatsHandler;
use store::contract_store::ContractStore;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = Arc::new(TrustConfig::from_env()?);
    tracing::info!(nats_url = %cfg.nats_url, "trust starting");

    let store = Arc::new(ContractStore::new());
    store.load_genesis(&cfg.genesis_contract_path)?;

    // Start HTTP (health/metrics) task
    tokio::spawn(http::health_metrics::start_http_server(cfg.metrics_host.clone(), cfg.metrics_port));

    // Start NATS handler
    let nats = NatsHandler::new(cfg.clone(), store.clone());
    nats.start().await?;

    // Block until shutdown
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown signal received");
    Ok(())
}
