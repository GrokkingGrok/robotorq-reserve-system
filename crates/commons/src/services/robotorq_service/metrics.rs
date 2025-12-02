//! Prometheus metrics handler (`/metrics`).
//!
//! Returns the Prometheus text exposition format by calling
//! `RoboTorqService::export_metrics()` on shared state.
//!
//! Typical use is to expose `GET /metrics` for scraping by Prometheus.
use super::RoboTorqService;
use axum::{
    extract::{Extension, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Handler for Prometheus metrics.
/// # Arguments
/// - `State(service)`: Shared `Arc<Mutex<S>>` where `S: RoboTorqService`.
/// # Returns
/// - `200 OK` and the Prometheus text exposition payload.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,ignore
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
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
///     let svc = Arc::new(Mutex::new(MySvc));
///     Router::new()
///         .route("/metrics", get(metrics_handler::<MySvc>))
///         .with_state(svc)
/// }
/// ```
pub async fn metrics_handler<S: RoboTorqService>(
    State(service): State<Arc<Mutex<S>>>,
    maybe_registry: Option<Extension<std::sync::Arc<dyn crate::util::metrics::MetricsRegistry>>>,
) -> impl IntoResponse {
    let svc = service.lock().await;
    let service_metrics = svc.export_metrics();
    let registry_metrics = maybe_registry.map(|Extension(reg)| reg.export_text()).unwrap_or_default();
    let body = format!("{}{}", service_metrics, registry_metrics);

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
