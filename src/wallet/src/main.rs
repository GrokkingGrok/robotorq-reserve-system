//! RoboTorq Wallet Service
//!
//! Minimal backend service that:
//! - Subscribes to NATS `distodam.units` topic
//! - Tracks wallet balances from RoboTorq units
//! - Exposes Prometheus metrics for Grafana
//! - Provides health check endpoint

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::StreamExt;
use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Phase3RoboTorqUnit {
    unit_id: String,
    joule_torq_total: f64,
    robo_torq_total: f64,
    contract_ids: Vec<String>,
    digger_ids: Vec<String>,
    // Add other fields as needed
}

#[derive(Debug, Clone, Default)]
struct WalletState {
    total_robo_torq: f64,
    total_joule_torq: f64,
    units_received: u64,
}

// ============================================================================
// Metrics
// ============================================================================

struct Metrics {
    units_received_total: IntCounter,
    robo_torq_balance: IntGauge,
    joule_torq_balance: IntGauge,
    registry: Registry,
}

impl Metrics {
    fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let units_received_total = IntCounter::new(
            "wallet_units_received_total",
            "Total number of RoboTorq units received",
        )?;
        registry.register(Box::new(units_received_total.clone()))?;

        let robo_torq_balance = IntGauge::new(
            "wallet_robo_torq_balance",
            "Current RoboTorq balance (in millitorq, 1 RT = 1000 millitorq)",
        )?;
        registry.register(Box::new(robo_torq_balance.clone()))?;

        let joule_torq_balance = IntGauge::new(
            "wallet_joule_torq_balance",
            "Current JouleTorq balance (in joules)",
        )?;
        registry.register(Box::new(joule_torq_balance.clone()))?;

        Ok(Self {
            units_received_total,
            robo_torq_balance,
            joule_torq_balance,
            registry,
        })
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    wallet: Arc<RwLock<WalletState>>,
    metrics: Arc<Metrics>,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn metrics_handler(State(state): State<AppState>) -> Response {
    let encoder = TextEncoder::new();
    let metric_families = state.metrics.registry.gather();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; version=0.0.4; charset=utf-8",
                )],
                buffer,
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response()
        }
    }
}

async fn balance_handler(State(state): State<AppState>) -> impl IntoResponse {
    let wallet = state.wallet.read().await;
    let response = serde_json::json!({
        "robo_torq": wallet.total_robo_torq,
        "joule_torq": wallet.total_joule_torq,
        "units_received": wallet.units_received,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap())
}

// ============================================================================
// NATS Subscriber
// ============================================================================

async fn start_nats_subscriber(
    nats_url: String,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to NATS at {}", nats_url);
    let client = async_nats::connect(&nats_url).await?;
    info!("Connected to NATS successfully");

    let mut subscriber = client.subscribe("distodam.units").await?;
    info!("Subscribed to distodam.units topic");

    while let Some(message) = subscriber.next().await {
        match serde_json::from_slice::<Phase3RoboTorqUnit>(&message.payload) {
            Ok(unit) => {
                info!(
                    "Received unit: {} - RT: {}, JT: {}",
                    unit.unit_id, unit.robo_torq_total, unit.joule_torq_total
                );

                // Update wallet state
                let mut wallet = state.wallet.write().await;
                wallet.total_robo_torq += unit.robo_torq_total;
                wallet.total_joule_torq += unit.joule_torq_total;
                wallet.units_received += 1;

                // Update metrics
                state.metrics.units_received_total.inc();
                state
                    .metrics
                    .robo_torq_balance
                    .set((wallet.total_robo_torq * 1000.0) as i64); // Convert to millitorq
                state
                    .metrics
                    .joule_torq_balance
                    .set(wallet.total_joule_torq as i64);

                info!(
                    "Updated balance - RT: {:.6}, JT: {:.2}, Units: {}",
                    wallet.total_robo_torq, wallet.total_joule_torq, wallet.units_received
                );
            }
            Err(e) => {
                error!("Failed to deserialize unit: {}", e);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "wallet=info,tower_http=info".to_string()),
        )
        .init();

    info!("Starting RoboTorq Wallet Service v0.1.0");

    // Configuration from environment
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string());

    info!("Configuration:");
    info!("  NATS URL: {}", nats_url);
    info!("  HTTP Port: {}", http_port);

    // Initialize metrics
    let metrics = Arc::new(Metrics::new()?);
    info!("Metrics initialized");

    // Initialize application state
    let state = AppState {
        wallet: Arc::new(RwLock::new(WalletState::default())),
        metrics,
    };

    // Start NATS subscriber in background
    let nats_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = start_nats_subscriber(nats_url, nats_state).await {
            error!("NATS subscriber error: {}", e);
        }
    });

    // Build HTTP router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/balance", get(balance_handler))
        .with_state(state);

    // Start HTTP server
    let addr = format!("0.0.0.0:{}", http_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Wallet service listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
