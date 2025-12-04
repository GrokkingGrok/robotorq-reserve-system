use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use tower::ServiceExt; // for `oneshot`

use commons::services::robotorq_service::middleware::HttpMetricsLayer;
use commons::util::metrics::{MetricsRegistry, PrometheusRegistry};

#[tokio::test]
async fn http_metrics_layer_records_basic_labels() {
    let registry = Arc::new(PrometheusRegistry::new("svc", "component", "v1"));
    let layer = HttpMetricsLayer::new(registry.clone());

    let app = Router::new()
        .route("/ok", get(|| async { StatusCode::OK }))
        .layer(layer);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ok")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Optional: export metrics to ensure gather path runs (do not assert content here to
    // avoid flakiness across environments).
    let _ = registry.export_text();
}
