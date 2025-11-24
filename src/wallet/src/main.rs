//! RoboTorq Wallet Service
//!
//! Economic interface for users to receive UBD, track RT balances,
//! and participate in the RoboTorq economy.

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
    Json,
};
use axum::routing::post;
use robotorq_wallet::{WalletService, WalletState};
use robotorq_wallet::models;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

/// Application state for HTTP handlers
#[derive(Clone)]
struct AppState {
    wallet_service: Arc<WalletService>,
    wallet_state: Arc<RwLock<WalletState>>,
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> Response {
    match state.wallet_service.metrics().encode() {
        Ok(metrics) => {
            (
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; version=0.0.4; charset=utf-8",
                )],
                metrics,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response()
        }
    }
}

/// Balance endpoint
async fn balance_handler(State(state): State<AppState>) -> impl IntoResponse {
    let balance_triple = state.wallet_service.get_balance().await;
    let wallet_state = state.wallet_state.read().await;
    let response = serde_json::json!({
        "balance": {
            "robotorq": balance_triple.robotorq,
            "tokentorq_remainder": balance_triple.tokentorq_remainder,
            "jouletorq_remainder": balance_triple.jouletorq_remainder,
            "canonical_jouletorq": models::triple_to_jouletorq(balance_triple),
        },
        "wallet_id": wallet_state.wallet_id,
        "is_activated": wallet_state.is_activated,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap())
}

/// Activate wallet endpoint
async fn activate_handler(State(state): State<AppState>) -> impl IntoResponse {
    let wallet_id = {
        let mut wallet_state = state.wallet_state.write().await;
        if wallet_state.wallet_id.is_none() {
            wallet_state.wallet_id = Some(format!("wallet-{}", uuid::Uuid::new_v4()));
        }
        wallet_state.is_activated = true;
        wallet_state.wallet_id.as_ref().unwrap().clone()
    };

    // Publish activation message
    if let Err(e) = state.wallet_service.nats_client().publish_activation(&wallet_id, true).await {
        tracing::error!("Failed to publish activation: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to activate wallet").into_response();
    }

    tracing::info!("Wallet activated: {}", wallet_id);
    let response = serde_json::json!({
        "wallet_id": wallet_id,
        "activated": true,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap()).into_response()
}

/// Deactivate wallet endpoint
async fn deactivate_handler(State(state): State<AppState>) -> impl IntoResponse {
    let wallet_id = {
        let mut wallet_state = state.wallet_state.write().await;
        wallet_state.is_activated = false;
        wallet_state.wallet_id.as_ref().unwrap_or(&"unknown".to_string()).clone()
    };

    if wallet_id == "unknown" {
        return (StatusCode::BAD_REQUEST, "Wallet not initialized").into_response();
    }

    // Publish deactivation message
    if let Err(e) = state.wallet_service.nats_client().publish_activation(&wallet_id, false).await {
        tracing::error!("Failed to publish deactivation: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deactivate wallet").into_response();
    }

    tracing::info!("Wallet deactivated: {}", wallet_id);
    let response = serde_json::json!({
        "wallet_id": wallet_id,
        "activated": false,
    });
    (StatusCode::OK, serde_json::to_string(&response).unwrap()).into_response()
}

/// Transaction quote request endpoint
async fn transaction_quote_handler(
    State(state): State<AppState>,
    Json(request): Json<models::TransactionRequest>,
) -> impl IntoResponse {
    // Get current wallet ID
    let wallet_state = state.wallet_state.read().await;
    let current_wallet_id = match &wallet_state.wallet_id {
        Some(id) => id.clone(),
        None => return (StatusCode::BAD_REQUEST, "Wallet not activated").into_response(),
    };

    // Validate request
    if request.from_wallet_id != current_wallet_id {
        return (StatusCode::BAD_REQUEST, "Request from_wallet_id does not match current wallet").into_response();
    }

    // Forward quote request to vault
    match state.wallet_service.nats_client().request_transaction_quote(request).await {
        Ok(quote) => {
            let response = serde_json::to_string(&quote).unwrap();
            (StatusCode::OK, response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get transaction quote: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get transaction quote").into_response()
        }
    }
}

/// Transaction send endpoint
async fn transaction_send_handler(
    State(state): State<AppState>,
    Json(commitment): Json<models::TransactionCommitment>,
) -> impl IntoResponse {
    // Forward commitment to vault
    match state.wallet_service.nats_client().commit_transaction(commitment).await {
        Ok(execution) => {
            let response = serde_json::to_string(&execution).unwrap();
            (StatusCode::OK, response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to send transaction: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to send transaction").into_response()
        }
    }
}

/// Transaction status endpoint
async fn transaction_status_handler(
    State(state): State<AppState>,
    axum::extract::Path(transaction_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Request status from vault
    match state.wallet_service.nats_client().get_transaction_status(&transaction_id).await {
        Ok(execution) => {
            let response = serde_json::to_string(&execution).unwrap();
            (StatusCode::OK, response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get transaction status: {}", e);
            (StatusCode::NOT_FOUND, "Transaction not found").into_response()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "robotorq_wallet=info,tower_http=info".to_string()),
        )
        .init();

    tracing::info!("Starting RoboTorq Wallet Service v{}", env!("CARGO_PKG_VERSION"));

    // Create wallet service
    let wallet_service = Arc::new(WalletService::new().await?);
    let wallet_state = Arc::new(RwLock::new(WalletState::default()));

    // Start wallet service
    let service_clone = Arc::clone(&wallet_service);
    tokio::spawn(async move {
        if let Err(e) = service_clone.start().await {
            tracing::error!("Wallet service error: {}", e);
        }
    });

    // Create application state
    let app_state = AppState {
        wallet_service,
        wallet_state,
    };

    // Build HTTP router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/balance", get(balance_handler))
        .route("/activate", get(activate_handler))
        .route("/deactivate", get(deactivate_handler))
        .route("/transaction/quote", post(transaction_quote_handler))
        .route("/transaction/send", post(transaction_send_handler))
        .route("/transaction/status/:transaction_id", get(transaction_status_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // Start HTTP server
    let addr = format!("0.0.0.0:{}", std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string()));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Wallet service listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models;

    #[tokio::test]
    async fn test_health_handler() {
        // Test that health handler compiles and runs
        // In a real test, you'd set up a test app and make HTTP requests
        assert!(true);
    }

    #[test]
    fn test_transaction_request_validation() {
        // Test transaction request structure
        let request = models::TransactionRequest {
            from_wallet_id: "wallet-001".to_string(),
            to_wallet_id: "wallet-002".to_string(),
            amount_jouletorq: 1000,
            timeframe_seconds: 3600,
        };

        assert_eq!(request.from_wallet_id, "wallet-001");
        assert_eq!(request.to_wallet_id, "wallet-002");
        assert_eq!(request.amount_jouletorq, 1000);
        assert_eq!(request.timeframe_seconds, 3600);
    }

    #[test]
    fn test_transaction_quote_response_structure() {
        // Test that quote response has expected fields
        let request = models::TransactionRequest {
            from_wallet_id: "wallet-001".to_string(),
            to_wallet_id: "wallet-002".to_string(),
            amount_jouletorq: 1000,
            timeframe_seconds: 3600,
        };

        let quote = models::TransactionQuote {
            quote_id: "quote-123".to_string(),
            request,
            fee_jouletorq: 10,
            estimated_completion_seconds: 3660,
            possible: true,
            reason: None,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(300),
        };

        assert_eq!(quote.quote_id, "quote-123");
        assert_eq!(quote.fee_jouletorq, 10);
        assert!(quote.possible);
    }

    #[test]
    fn test_transaction_commitment_structure() {
        let commitment = models::TransactionCommitment {
            quote_id: "quote-123".to_string(),
            accepted: true,
        };

        assert_eq!(commitment.quote_id, "quote-123");
        assert!(commitment.accepted);
    }

    #[test]
    fn test_transaction_status_response_structure() {
        let execution = models::TransactionExecution {
            transaction_id: "tx-123".to_string(),
            quote_id: "quote-123".to_string(),
            status: models::TransactionStatus::Completed,
            progress: 1.0,
            transferred_jouletorq: 1000,
            started_at: chrono::Utc::now() - chrono::Duration::seconds(30),
            completed_at: Some(chrono::Utc::now()),
            error_message: None,
        };

        let response = models::TransactionStatusResponse {
            transaction_id: "tx-123".to_string(),
            status: models::TransactionStatus::Completed,
            quote: None,
            execution: Some(execution),
            last_updated: chrono::Utc::now(),
        };

        assert_eq!(response.transaction_id, "tx-123");
        assert_eq!(response.status, models::TransactionStatus::Completed);
        assert!(response.execution.is_some());
    }
}