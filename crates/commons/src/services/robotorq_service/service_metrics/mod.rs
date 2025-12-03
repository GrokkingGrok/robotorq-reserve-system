//! Prometheus metrics handler (`/metrics`).
//!
//! Returns the Prometheus text exposition format by calling
//! `RoboTorqService::export_metrics()` on shared state.
//!
//! Typical use is to expose `GET /metrics` for scraping by Prometheus.
use super::RoboTorqService;
use crate::util::error::ServiceError;
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
/// #     // Note: the public `RoboTorqService` trait uses `ServiceError` for
/// #     // service-level errors. Implementations may still use domain-specific
/// #     // errors internally and map them to `ServiceError` at the boundary.
/// #     fn health_check(&self) -> Result<String, commons::util::error::ServiceError> { Ok("OK".into()) }
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
    // Use an internal fallible helper returning `ServiceError` so internals
    // can adopt the typed error progressively.
    let body = match export_metrics_text(Arc::clone(&service), maybe_registry).await {
        Ok(b) => b,
        Err(err) => {
            tracing::error!(error = ?err, "failed to build metrics text");
            // Return an empty body on failure but 200 to avoid scraping disruption.
            String::new()
        }
    };

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

async fn export_metrics_text<S: RoboTorqService>(
    service: Arc<Mutex<S>>,
    maybe_registry: Option<Extension<std::sync::Arc<dyn crate::util::metrics::MetricsRegistry>>>,
) -> Result<String, ServiceError> {
    let svc = service.lock().await;
    let service_metrics = crate::services::robotorq_service::RoboTorqService::export_metrics(&*svc);
    let registry_metrics = maybe_registry
        .map(|Extension(reg)| reg.export_text())
        .unwrap_or_default();
    Ok(format!("{}{}", service_metrics, registry_metrics))
}
