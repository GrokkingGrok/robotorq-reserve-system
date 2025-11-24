use anyhow::Result;
use tracing::{info, error};

mod config;
mod nats_client;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Rust Mint service");

    // Load config
    let config = config::MintConfig::from_env()?;

    // Connect to NATS
    let nats_client = nats_client::connect(&config.nats_url).await?;

    info!("Connected to NATS at {}", config.nats_url);

    // TODO: Start ingot subscriber task

    // For now, just keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Mint service");

    Ok(())
}