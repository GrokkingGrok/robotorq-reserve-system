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

// Phase3RoboTorqUnit - The actual RT unit certificate from Mint
// Each unit is a complete cryptographic certificate with merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Phase3RoboTorqUnit {
    unit_id: String,
    merkle_root: String,
    tree_height: i32,
    robo_stake_total: f64,
    contract_ids: Vec<String>,
    digger_ids: Vec<String>,
    merkle_proof_api: String,
    minted_at: String,
    signature: Option<String>,
    public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletDistributionMessage {
    wallet_id: String,
    rt_unit: Phase3RoboTorqUnit,  // Send the actual certificate!
    timestamp: String,
    disto_balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletActivationMessage {
    wallet_id: String,
    activate: bool,
    requested_at: String,
}

#[derive(Debug, Clone, Default)]
struct WalletState {
    // Store actual RT unit certificates (not counts!)
    // Each certificate contains merkle_root proof and full provenance
    rt_units: Vec<Phase3RoboTorqUnit>,
    wallet_id: String,
    is_activated: bool,
}

// ============================================================================
// Metrics
// ============================================================================

struct Metrics {
    rt_units_received_total: IntCounter,
    rt_units_balance: IntGauge,
    registry: Registry,
}

impl Metrics {
    fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let rt_units_received_total = IntCounter::new(
            "wallet_rt_units_received_total",
            "Total number of RT units received from distributions",
        )?;
        registry.register(Box::new(rt_units_received_total.clone()))?;

        let rt_units_balance = IntGauge::new(
            "wallet_rt_units_balance",
            "Current RT units balance (discrete units, not floats)",
        )?;
        registry.register(Box::new(rt_units_balance.clone()))?;

        Ok(Self {
            rt_units_received_total,
            rt_units_balance,
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
        "rt_unit_count": wallet.rt_units.len(),
        "rt_units": wallet.rt_units.iter().map(|u| serde_json::json!({
            "unit_id": u.unit_id,
            "merkle_root": u.merkle_root,
            "minted_at": u.minted_at,
            "contract_ids": u.contract_ids,
            "digger_ids": u.digger_ids,
        })).collect::<Vec<_>>(),
        "wallet_id": wallet.wallet_id,
        "is_activated": wallet.is_activated,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap())
}

async fn activate_handler(State(state): State<AppState>) -> impl IntoResponse {
    let wallet_id = {
        let mut wallet = state.wallet.write().await;
        if wallet.wallet_id.is_empty() {
            wallet.wallet_id = format!("wallet-{}", uuid::Uuid::new_v4());
        }
        wallet.is_activated = true;
        wallet.wallet_id.clone()
    };

    // Publish activation message to NATS
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    
    match async_nats::connect(&nats_url).await {
        Ok(client) => {
            let activation_msg = WalletActivationMessage {
                wallet_id: wallet_id.clone(),
                activate: true,
                requested_at: chrono::Utc::now().to_rfc3339(),
            };
            
            match serde_json::to_vec(&activation_msg) {
                Ok(msg_data) => {
                    if let Err(e) = client.publish("wallet.activate", msg_data.into()).await {
                        error!("Failed to publish activation: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to activate wallet").into_response();
                    }
                    info!("Wallet activated: {}", wallet_id);
                }
                Err(e) => {
                    error!("Failed to serialize activation message: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to activate wallet").into_response();
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to NATS: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to activate wallet").into_response();
        }
    }

    let response = serde_json::json!({
        "wallet_id": wallet_id,
        "activated": true,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap()).into_response()
}

async fn deactivate_handler(State(state): State<AppState>) -> impl IntoResponse {
    let wallet_id = {
        let mut wallet = state.wallet.write().await;
        wallet.is_activated = false;
        wallet.wallet_id.clone()
    };

    if wallet_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Wallet not initialized").into_response();
    }

    // Publish deactivation message to NATS
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    
    match async_nats::connect(&nats_url).await {
        Ok(client) => {
            let activation_msg = WalletActivationMessage {
                wallet_id: wallet_id.clone(),
                activate: false,
                requested_at: chrono::Utc::now().to_rfc3339(),
            };
            
            match serde_json::to_vec(&activation_msg) {
                Ok(msg_data) => {
                    if let Err(e) = client.publish("wallet.activate", msg_data.into()).await {
                        error!("Failed to publish deactivation: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deactivate wallet").into_response();
                    }
                    info!("Wallet deactivated: {}", wallet_id);
                }
                Err(e) => {
                    error!("Failed to serialize deactivation message: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deactivate wallet").into_response();
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to NATS: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deactivate wallet").into_response();
        }
    }

    let response = serde_json::json!({
        "wallet_id": wallet_id,
        "activated": false,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap()).into_response()
}

// ============================================================================
// Mint Verification Client
// ============================================================================

// Verify certificate with Mint to ensure it's legitimate
async fn verify_certificate_with_mint(
    merkle_root: &str,
    unit_id: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let mint_verification_url = std::env::var("MINT_VERIFICATION_URL")
        .unwrap_or_else(|_| "http://mint:8081/verify/certificate".to_string());

    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "merkle_root": merkle_root,
        "unit_id": unit_id,
    });

    let response = client
        .post(&mint_verification_url)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    if response.status().is_success() {
        let verification_result: serde_json::Value = response.json().await?;
        if let Some(valid) = verification_result.get("valid").and_then(|v| v.as_bool()) {
            return Ok(valid);
        }
    }

    Ok(false)
}

// ============================================================================
// NATS Subscribers
// ============================================================================

// Wallet ONLY subscribes to wallet.distribution (RT units from DistoDam)
// Wallet NEVER receives distodam.units (Phase3RoboTorqUnit with RoboStake)

async fn start_distribution_subscriber(
    nats_url: String,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to NATS for distribution at {}", nats_url);
    let client = async_nats::connect(&nats_url).await?;
    info!("Connected to NATS successfully");

    let mut subscriber = client.subscribe("wallet.distribution").await?;
    info!("Subscribed to wallet.distribution topic");

    while let Some(message) = subscriber.next().await {
        match serde_json::from_slice::<WalletDistributionMessage>(&message.payload) {
            Ok(dist_msg) => {
                info!(
                    "Received RT certificate: {} (merkle_root: {})",
                    dist_msg.rt_unit.unit_id, dist_msg.rt_unit.merkle_root
                );

                // FIRST: Verify certificate with Mint before accepting
                info!(
                    "Verifying certificate with Mint: {} (merkle_root: {})",
                    dist_msg.rt_unit.unit_id, dist_msg.rt_unit.merkle_root
                );

                match verify_certificate_with_mint(
                    &dist_msg.rt_unit.merkle_root,
                    &dist_msg.rt_unit.unit_id,
                )
                .await
                {
                    Ok(true) => {
                        info!(
                            "✅ Certificate VERIFIED by Mint: {} (merkle_root: {})",
                            dist_msg.rt_unit.unit_id, dist_msg.rt_unit.merkle_root
                        );

                        // Store the actual RT unit certificate
                        let mut wallet = state.wallet.write().await;
                        wallet.rt_units.push(dist_msg.rt_unit.clone());

                        // Update metrics
                        state.metrics.rt_units_received_total.inc();
                        state.metrics.rt_units_balance.set(wallet.rt_units.len() as i64);

                        info!(
                            "RT certificate stored - Total certificates: {}, Unit ID: {}, DistoVault balance: {:.6} RT",
                            wallet.rt_units.len(), dist_msg.rt_unit.unit_id, dist_msg.disto_balance
                        );
                    }
                    Ok(false) => {
                        error!(
                            "❌ Certificate REJECTED by Mint: {} (merkle_root: {})",
                            dist_msg.rt_unit.unit_id, dist_msg.rt_unit.merkle_root
                        );
                        error!("Certificate not found in Mint records - potential fraud!");
                    }
                    Err(e) => {
                        error!(
                            "⚠️  Failed to verify certificate with Mint: {} (merkle_root: {}), error: {}",
                            dist_msg.rt_unit.unit_id, dist_msg.rt_unit.merkle_root, e
                        );
                        error!("Verification network error - certificate NOT stored");
                    }
                }
            }
            Err(e) => {
                error!("Failed to deserialize distribution message: {}", e);
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

    // Start ONLY the distribution subscriber (wallet doesn't need distodam.units)
    // Wallet receives RT units from DistoDam via wallet.distribution topic
    let dist_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = start_distribution_subscriber(nats_url, dist_state).await {
            error!("NATS distribution subscriber error: {}", e);
        }
    });

    // Build HTTP router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/balance", get(balance_handler))
        .route("/activate", get(activate_handler))
        .route("/deactivate", get(deactivate_handler))
        .with_state(state);

    // Start HTTP server
    let addr = format!("0.0.0.0:{}", http_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Wallet service listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
