//! Liveness probe handler (`/healthz`).
//!
//! Invokes `RoboTorqService::health_check()` on the shared service state:
//! - Returns HTTP 200 with the health message when healthy.
//! - Returns HTTP 500 with error details when unhealthy.
//!
//! Use this to signal “the process is alive and the core dependencies are
//! reachable” to orchestrators like Kubernetes.
use std::sync::Arc;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use crate::services::http::RoboTorqService;

/// Handler for liveness health checks.
pub async fn health_handler<S: RoboTorqService>(
    State(service): State<Arc<S>>,
) -> impl IntoResponse {
    // Returns 200 OK when healthy, 500 otherwise
    match service.health_check() {
        Ok(message) => (StatusCode::OK, message),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Health check failed: {:?}", e)),
    }
}
