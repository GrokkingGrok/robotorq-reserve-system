use anyhow::Result;
use std::sync::Arc;

mod config;
mod handlers;
mod http;
mod metrics;
mod models;
mod store;

use config::TrustConfig;
use handlers::nats_handler::NatsHandler;
use store::contract_store::ContractStore;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Arc::new(TrustConfig::from_env()?);

    // Initialize tracing subscriber with configured log level (map string -> Level)
    let level = match cfg.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" | "warning" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    tracing::info!(nats_url = %cfg.nats_url, "trust starting");

    let store = Arc::new(ContractStore::new());
    store.load_genesis(&cfg.genesis_contract_path)?;
    // Mark service readiness after successful genesis load
    http::health_metrics::set_ready();

    // Create a shutdown watch channel. Sending `true` will signal shutdown.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start HTTP (health/metrics) task with graceful shutdown receiver
    let http_shutdown = shutdown_rx.clone();
    let http_task = tokio::spawn(http::health_metrics::start_http_server(cfg.http_host.clone(), cfg.http_port, http_shutdown));

    // Start NATS handler in background with shutdown receiver
    let nats = NatsHandler::new(cfg.clone(), store.clone());
    let nats_shutdown = shutdown_rx.clone();
    let nats_task = tokio::spawn(async move {
        if let Err(e) = nats.start(nats_shutdown).await {
            tracing::error!(error = %e, "NATS handler failed");
        }
    });

    // Wait for Ctrl-C
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown signal received, notifying tasks");

    // Signal shutdown to tasks
    let _ = shutdown_tx.send(true);

    // Give tasks a short grace period to finish (from config)
    let grace = std::time::Duration::from_secs(cfg.shutdown_grace_seconds);
    let _ = tokio::time::timeout(grace, async {
        let _ = nats_task.await;
        let _ = http_task.await;
    }).await;

    tracing::info!("shutdown complete");
    Ok(())
}
