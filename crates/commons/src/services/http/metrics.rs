//! Prometheus metrics handler (`/metrics`).
//!
//! Returns the Prometheus text exposition format by calling
//! `RoboTorqService::export_metrics()` on shared state.
//!
//! Typical use is to expose `GET /metrics` for scraping by Prometheus.
use std::sync::Arc;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use crate::services::http::RoboTorqService;

/// Handler for Prometheus metrics.
/// # Arguments
/// - `State(service)`: Shared `Arc<S>` where `S: RoboTorqService`.
/// # Returns
/// - `200 OK` and the Prometheus text exposition payload.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,ignore
/// use std::sync::Arc;
/// use axum::{routing::get, Router};
/// use commons::services::http::metrics::metrics_handler;
/// use commons::services::http::RoboTorqService;
///
/// # struct MySvc; /* impl RoboTorqService for MySvc { /* ... */ } */
/// # impl commons::services::http::RoboTorqService for MySvc {
/// #     fn export_metrics(&self) -> String { "# HELP demo demo\n".into() }
/// #     fn health_check(&self) -> Result<String, commons::util::error::InvariantError> { Ok("OK".into()) }
/// #     fn shutdown<'a>(&'a self) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
/// #     fn initialize<'a>(&'a mut self, _cfg: &commons::util::config::RoboTorqConfig) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
/// # }
///
/// async fn router() -> Router {
///     let svc = Arc::new(MySvc);
///     Router::new()
///         .route("/metrics", get(metrics_handler::<MySvc>))
///         .with_state(svc)
/// }
/// ```
pub async fn metrics_handler<S: RoboTorqService>(
    State(service): State<Arc<S>>,
) -> impl IntoResponse {
    
    (StatusCode::OK, service.export_metrics())
}
