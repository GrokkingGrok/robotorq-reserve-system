// Digger v0.2.0 - Pure Backend (No Tauri)
// Phase 1: HTTP Server + SQLite Storage + Hash-Only Transmission

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod contract_state;
mod http_api;
mod jtu_hasher;
mod jtu_storage;

use config::DiggerConfig;
use contract_state::ContractStateManager;
use jtu_storage::JtuStorageManager;
use http_api::{ApiState, create_router};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "digger=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🤖 Digger v0.2.0 starting...");

    // Load config
    let config = match DiggerConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("Digger ID: {}", config.digger_id);
    tracing::info!("HTTP Port: {}", config.http_port);
    tracing::info!("Storage Path: {}", config.storage_path.display());
    tracing::info!("Batch Interval: {} seconds", config.batch_interval_sec);

    // Initialize managers
    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to initialize storage manager");

    // Create API state
    let state = ApiState::new(config.clone(), contract_manager, storage_manager);

    // Create router with all API endpoints
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    tracing::info!("🚀 Digger HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    tracing::info!("✅ Server ready with full API!");
    
    axum::serve(listener, app)
        .await
        .unwrap();
    
    tracing::info!("Server shutdown");
}
