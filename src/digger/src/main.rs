// Digger v0.2.0 - Pure Backend (No Tauri)
// Phase 1: HTTP Server + SQLite Storage + Hash-Only Transmission

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod jtu_hasher;
mod jtu_storage;

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

    // Build router (placeholder routes)
    let app = Router::new()
        .route("/", get(root))
        .route("/status", get(status));

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 9000));
    tracing::info!("🚀 Digger HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    tracing::info!("✅ Server ready, accepting connections...");
    
    axum::serve(listener, app)
        .await
        .unwrap();
    
    tracing::info!("Server shutdown");
}

async fn root() -> &'static str {
    "Digger v0.2.0 - Pure Backend"
}

async fn status() -> &'static str {
    "OK"
}
