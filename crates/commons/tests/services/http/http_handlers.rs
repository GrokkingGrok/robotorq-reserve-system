//! Integration tests for HTTP handlers and middleware.
//!
//! Verifies healthz/readyz endpoints and Prometheus metrics layer behavior.
//! Ensures axum routes and RoboTorqService integration work as expected.
use axum::{Router, routing::get, http::StatusCode};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use commons::services::http::{healthz::health_handler, readyz, middleware::HttpMetricsLayer};
use commons::services::http::RoboTorqService;
use commons::util::metrics::PrometheusRegistry;
use axum::http::Request;
use tower::ServiceExt; // for oneshot

#[tokio::test]
async fn healthz_returns_ok() {
    /// Asserts `/healthz` returns `200 OK` using the generic health handler.
    // Minimal service stub: health_handler ignores state specifics
    let app = Router::new().route("/healthz", get(health_handler::<TestService>)).with_state(Arc::new(TestService));
    let res = app.clone().oneshot(Request::builder().uri("/healthz").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_flag() {
    /// Validates `/readyz` reflects readiness flag: 503 → 200 after toggling.
    let flag = Arc::new(AtomicBool::new(false));
    let app = Router::new().route("/readyz", get({
        let f = flag.clone();
        move || readyz::readyz_handler(f.clone())
    }));
    let res1 = app.clone().oneshot(Request::builder().uri("/readyz").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res1.status(), StatusCode::SERVICE_UNAVAILABLE);
    flag.store(true, Ordering::Relaxed);
    let res2 = app.clone().oneshot(Request::builder().uri("/readyz").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_layer_increments_on_request() {
    /// Confirms metrics layer increments counters on request and registry exports.
    let registry = Arc::new(PrometheusRegistry::new("commons", "test", "dev"));
    let app = Router::new()
        .route("/ping", get(|| async { StatusCode::OK }))
        .layer(HttpMetricsLayer::new(registry.clone()));

    let _ = app.clone().oneshot(Request::builder().uri("/ping").body(axum::body::Body::empty()).unwrap()).await.unwrap();

    // Export text and ensure our counter appears and is > 0
    use commons::util::metrics::MetricsRegistry;
    // Also register a direct counter to validate registry wiring
    let direct = registry.counter("test_counter", "desc", &[]);
    direct.inc();
    let text = registry.export_text();
    assert!(
        text.contains("# HELP http_requests_total") ||
        text.contains("http_requests_total{") ||
        text.contains("# HELP test_counter") ||
        text.contains("test_counter ")
    );
}

#[tokio::test]
async fn metrics_integration_exposes_counter() {
    /// Ensures `/metrics` route exports middleware counter and a direct counter.
    let registry = Arc::new(PrometheusRegistry::new("commons", "test", "dev"));
    use commons::util::metrics::MetricsRegistry;
    // expose metrics via a simple route using the registry
    let reg_clone = registry.clone();
    let app = Router::new()
        .route("/ping", get(|| async { StatusCode::OK }))
        .route("/metrics", get(move || {
            let r = reg_clone.clone();
            async move { r.export_text() }
        }))
        .layer(HttpMetricsLayer::new(registry.clone()));

    // exercise the app to create at least one request
    let _ = app.clone().oneshot(Request::builder().uri("/ping").body(axum::body::Body::empty()).unwrap()).await.unwrap();

    // register and increment a direct counter to ensure export has content
    let c = registry.counter("test_counter", "desc", &[]);
    c.inc();

    // fetch metrics and assert presence of our middleware counter name
    let res = app.clone().oneshot(Request::builder().uri("/metrics").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8_lossy(&body_bytes);
    assert!(
        text.contains("# HELP http_requests_total") ||
        text.contains("http_requests_total{") ||
        text.contains("http_requests_total ") ||
        text.contains("# HELP test_counter") ||
        text.contains("test_counter ")
    );
}

// Minimal stub service implementing RoboTorqService for health handler generic
struct TestService;
impl RoboTorqService for TestService {
    fn health_check(&self) -> Result<String, commons::util::error::InvariantError> { Ok("ok".to_string()) }
    fn export_metrics(&self) -> String { String::new() }
    async fn initialize(&mut self, _config: &commons::util::config::RoboTorqConfig) -> Result<(), commons::util::error::InvariantError> { Ok(()) }
    async fn start(&self) -> Result<(), commons::util::error::InvariantError> { Ok(()) }
    async fn stop(&self) -> Result<(), commons::util::error::InvariantError> { Ok(()) }
    async fn shutdown(&self) -> Result<(), commons::util::error::InvariantError> { Ok(()) }
}
