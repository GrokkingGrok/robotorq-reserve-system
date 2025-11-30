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
    //! # Arguments
    //! - `State(service)`: Shared `Arc<S>` where `S: RoboTorqService`.
    //!
    //! # Returns
    //! - `200 OK` and a short message when healthy.
    //! - `500 INTERNAL_SERVER_ERROR` and details when unhealthy.
    //!
    //! # Panics
    //! - Not expected to panic.
    //!
    //! # Examples
    //! ```rust,ignore
    //! use std::sync::Arc;
    //! use axum::{routing::get, Router};
    //! use commons::services::http::healthz::health_handler;
    //! use commons::services::http::RoboTorqService;
    //!
    //! // Suppose `MySvc` implements `RoboTorqService` and is `Send + Sync`.
    //! # struct MySvc; /* impl RoboTorqService for MySvc { /* ... */ } */
    //! # impl commons::services::http::RoboTorqService for MySvc {
    //! #     fn export_metrics(&self) -> String { String::new() }
    //! #     fn health_check(&self) -> Result<String, commons::util::error::InvariantError> { Ok("OK".into()) }
    //! #     fn shutdown<'a>(&'a self) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    //! #     fn initialize<'a>(&'a mut self, _cfg: &commons::util::config::RoboTorqConfig) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
    //! # }
    //!
    //! async fn router() -> Router {
    //!     let svc = Arc::new(MySvc);
    //!     Router::new()
    //!         .route("/healthz", get(health_handler::<MySvc>))
    //!         .with_state(svc)
    //! }
    //! ```
    match service.health_check() {
        Ok(message) => (StatusCode::OK, message),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Health check failed: {:?}", e)),
    }
}
