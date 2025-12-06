//! Liveness probe handler (`/healthz`).
//!
//! Overview
//! - Adapts `RoboTorqService::health_check()` to an Axum HTTP endpoint.
//! - Returns `200 OK` with the health message when healthy.
//! - Returns `500 Internal Server Error` with details when unhealthy.
//!
//! When to use
//! - Exposes a cheap "is the process alive and core dependencies reachable?" signal
//!   to orchestrators like Kubernetes. Pair with readiness endpoints for strict checks.
//!
//! Quick Start
//! ```no_run
//! use std::sync::Arc;
//! use axum::{routing::get, Router, Extension};
//! use commons::services::robotorq_service::healthz::{health_handler, HealthzMetrics};
//! use commons::util::metrics::PrometheusRegistry;
//! use tokio::sync::Mutex;
//!
//! // Your service implements RoboTorqService
//! struct MySvc;
//! impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
//! impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".into() } }
//! impl commons::services::robotorq_service::MetricsContributor for MySvc {}
//! impl commons::services::robotorq_service::RoboTorqService for MySvc {
//!     fn export_metrics(&self) -> String { String::new() }
//!     fn health_check(&self) -> Result<String, commons::util::error::ServiceError> { Ok("OK".into()) }
//!     async fn initialize(&mut self, _cfg: &commons::util::config::RoboTorqConfig) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
//!     async fn start(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
//!     async fn stop(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
//!     async fn shutdown(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
//! }
//!
//! let svc = Arc::new(Mutex::new(MySvc));
//! let registry = Arc::new(PrometheusRegistry::new("svc","gateway","v1"));
//! let health_metrics = Arc::new(HealthzMetrics::new(registry));
//! let app: Router<Arc<Mutex<MySvc>>> = Router::new()
//!     .route("/healthz", get(health_handler::<MySvc>))
//!     .with_state(svc)
//!     .layer(Extension(health_metrics));
//! # let _ = app;
//! ```

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
    ///
    /// # Arguments
    /// - `registry`: Shared metrics registry used to create counters for `/healthz`.
    ///
    /// # Returns
    /// - A `HealthzMetrics` instance with endpoint-specific counters.
    ///
    /// # Panics
    /// - This function does not panic.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use commons::util::metrics::PrometheusRegistry;
    /// use commons::services::robotorq_service::healthz::HealthzMetrics;
    /// let metrics = HealthzMetrics::new(Arc::new(PrometheusRegistry::new("svc","comp","v1")));
    /// # let _ = metrics;
    /// ```
    #[allow(clippy::needless_pass_by_value)]
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
///
/// # Arguments
/// - `service`: Shared, mutex-protected service implementing `RoboTorqService`.
/// - `maybe_metrics`: Optional `HealthzMetrics` shared via `Extension`.
///
/// # Returns
/// - `200 OK` with the service health message when healthy.
/// - `500 Internal Server Error` with details when unhealthy.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```no_run
/// use std::sync::Arc;
/// use axum::{routing::get, Router, Extension};
/// use commons::services::robotorq_service::healthz::{health_handler, HealthzMetrics};
/// use commons::util::metrics::PrometheusRegistry;
/// use tokio::sync::Mutex;
///
/// struct MySvc;
/// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
/// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".into() } }
/// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
/// impl commons::services::robotorq_service::RoboTorqService for MySvc {
///     fn export_metrics(&self) -> String { String::new() }
///     fn health_check(&self) -> Result<String, commons::util::error::ServiceError> { Ok("OK".into()) }
///     async fn initialize(&mut self, _cfg: &commons::util::config::RoboTorqConfig) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
///     async fn start(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
///     async fn stop(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
///     async fn shutdown(&self) -> Result<(), commons::util::error::ServiceError> { Ok(()) }
/// }
/// let svc = Arc::new(Mutex::new(MySvc));
/// let registry = Arc::new(PrometheusRegistry::new("svc","gateway","v1"));
/// let metrics = Arc::new(HealthzMetrics::new(registry));
/// let app: Router<Arc<Mutex<MySvc>>> = Router::new()
///     .route("/healthz", get(health_handler::<MySvc>))
///     .with_state(svc)
///     .layer(Extension(metrics));
/// # let _ = app;
/// ```
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
            format!("Health check failed: {e:?}"),
        ),
    }
}
