//! HTTP observability middleware: request counters, durations, in-flight gauge.
use crate::util::metrics::{LabeledCounter, LabeledHistogram, MetricGauge, MetricsRegistry};
use axum::http::Request;
use axum::response::Response;
use axum::extract::MatchedPath;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use std::future::Future;
use std::pin::Pin;
use tower::{Layer, Service};

/// Axum layer that wires HTTP metrics into the request pipeline.
#[derive(Clone)]
pub struct HttpMetricsLayer {
    registry: Arc<dyn MetricsRegistry>,
}

impl HttpMetricsLayer {
    /// Create a new metrics layer using the provided registry.
    pub fn new(registry: Arc<dyn MetricsRegistry>) -> Self {
        Self { registry }
    }
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let requests_total = self
            .registry
            .counter_vec("http_requests_total", "Total HTTP requests", &["method", "status", "path"]);
        let inflight =
            self.registry
                .gauge("http_inflight_requests", "In-flight HTTP requests", &[]);
        let durations = self.registry.histogram_vec(
            "http_request_duration_seconds",
            "HTTP request durations in seconds",
            &["method", "status", "path"],
            Some(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        );
        let errors_total = self
            .registry
            .counter_vec("http_errors_total", "HTTP error responses (>=400)", &["method", "status_class", "path"]);

        HttpMetricsService {
            inner,
            requests_total: requests_total.into(),
            inflight: inflight.into(),
            durations: durations.into(),
            errors_total: errors_total.into(),
        }
    }
}

/// Service wrapper that records request counters, inflight gauge, and durations.
#[derive(Clone)]
pub struct HttpMetricsService<S> {
    inner: S,
    requests_total: Arc<dyn LabeledCounter + Send + Sync>,
    inflight: Arc<dyn MetricGauge + Send + Sync>,
    durations: Arc<dyn LabeledHistogram + Send + Sync>,
    errors_total: Arc<dyn LabeledCounter + Send + Sync>,
}

impl<S, B> Service<Request<B>> for HttpMetricsService<S>
where
    S: Service<Request<B>, Response = Response> + Send,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let start = Instant::now();
        self.inflight.inc();

        let method = req.method().as_str().to_string();
        // MatchedPath is set by axum Router; fallback to raw path if missing
        let path = req
            .extensions()
            .get::<MatchedPath>()
            .map(|mp| mp.as_str().to_string())
            .unwrap_or_else(|| req.uri().path().to_string());

        let fut = self.inner.call(req);

        let inflight = self.inflight.clone();
        let requests_total = self.requests_total.clone();
        let durations = self.durations.clone();
        let errors_total = self.errors_total.clone();

        Box::pin(async move {
            let result = fut.await;
            match &result {
                Ok(res) => {
                    let status_code = res.status().as_u16();
                    let status = status_code.to_string();
                    let labels = [("method", method.as_str()), ("status", status.as_str()), ("path", path.as_str())];
                    requests_total.inc(&labels);
                    durations.observe_duration(start.elapsed(), &labels);
                    if status_code >= 400 {
                        let class = if status_code >= 500 { "5xx" } else { "4xx" };
                        let elabels = [("method", method.as_str()), ("status_class", class), ("path", path.as_str())];
                        errors_total.inc(&elabels);
                    }
                }
                Err(_err) => {
                    // On inner service error, count as 5xx and record duration
                    let labels = [("method", method.as_str()), ("status", "500"), ("path", path.as_str())];
                    requests_total.inc(&labels);
                    durations.observe_duration(start.elapsed(), &labels);
                    let elabels = [("method", method.as_str()), ("status_class", "5xx"), ("path", path.as_str())];
                    errors_total.inc(&elabels);
                }
            }
            inflight.dec();
            result
        })
    }
}
