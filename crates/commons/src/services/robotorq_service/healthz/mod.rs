//! Liveness probe handler (`/healthz`).
//!
//! Invokes `RoboTorqService::health_check()` on the shared service state:
//! - Returns HTTP 200 with the health message when healthy.
//! - Returns HTTP 500 with error details when unhealthy.
//!
//! Use this to signal “the process is alive and the core dependencies are
//! reachable” to orchestrators like Kubernetes.

use crate::{
    services::robotorq_service::RoboTorqService,
    util::metrics::{MetricCounter, MetricsRegistry},
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// HTTP-level metrics for the `/healthz` endpoint.
///
/// These metrics capture HTTP traffic characteristics and are distinct from
/// service-level health metrics. Labels are injected via the shared registry.
pub struct HealthzMetrics {
    /// Total number of HTTP `/healthz` requests received.
    pub requests_total: Box<dyn MetricCounter + Send + Sync>,
}

impl HealthzMetrics {
    /// Construct HTTP health endpoint metrics from the shared registry.
    pub fn new(registry: Arc<dyn MetricsRegistry>) -> Self {
        let requests_total = registry.counter(
            "http_health_requests_total",
            "Total HTTP /healthz requests",
            &[],
        );
        Self { requests_total }
    }
}

/// Handler for liveness health checks.
pub async fn health_handler<S: RoboTorqService>(
    State(service): State<Arc<Mutex<S>>>,
    maybe_metrics: Option<Extension<Arc<HealthzMetrics>>>,
) -> impl IntoResponse {
    let svc = service.lock().await;
    if let Some(Extension(m)) = maybe_metrics.as_ref() {
        m.requests_total.inc();
    }
    // Returns 200 OK when healthy, 500 otherwise
    match svc.health_check() {
        Ok(message) => (StatusCode::OK, message),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Health check failed: {:?}", e),
        ),
    }
}
