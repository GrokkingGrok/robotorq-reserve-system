//! HTTP observability middleware: request counters, durations, in-flight gauge.
use crate::util::metrics::{LabeledCounter, LabeledHistogram, MetricGauge, MetricsRegistry};
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};
#[cfg(feature = "otlp")]
use tracing::Span;

#[cfg(feature = "otlp")]
use opentelemetry::trace::TraceContextExt;

/// Axum layer that wires HTTP metrics into the request pipeline.
///
/// Records request totals, in-flight counts, latencies, and error totals
/// using a provided `MetricsRegistry` implementation.
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use axum::{Router, routing::get};
/// use commons::util::metrics::PrometheusRegistry;
/// use commons::services::robotorq_service::middleware::HttpMetricsLayer;
///
/// let registry = Arc::new(PrometheusRegistry::new("svc","component","v1"));
/// let layer = HttpMetricsLayer::new(registry);
/// let app = Router::new()
///     .route("/health", get(|| async { "ok" }))
///     .layer(layer);
/// ```
#[derive(Clone)]
pub struct HttpMetricsLayer {
    registry: Arc<dyn MetricsRegistry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as AxumRequest;

    /// Verify that `insert_trace_context` places a `TraceContext` into the
    /// request extensions and that the inserted value is well-formed.
    #[test]
    fn insert_trace_context_populates_extensions() {
        let mut req: Request<()> = AxumRequest::builder().uri("/test").body(()).unwrap();
        insert_trace_context(&mut req);
        let tc = req
            .extensions()
            .get::<TraceContext>()
            .expect("TraceContext not inserted");
        // By default (no OTLP feature), ids are None.
        assert!(tc.trace_id.is_none());
        assert!(tc.span_id.is_none());
    }

    /// Calling `insert_trace_context` multiple times should be safe and idempotent.
    #[test]
    fn insert_trace_context_idempotent() {
        let mut req: Request<()> = AxumRequest::builder().uri("/again").body(()).unwrap();
        insert_trace_context(&mut req);
        insert_trace_context(&mut req);
        let tc = req
            .extensions()
            .get::<TraceContext>()
            .expect("TraceContext missing after repeated insert");
        assert!(tc.trace_id.is_none());
        assert!(tc.span_id.is_none());
    }
}

impl HttpMetricsLayer {
    /// Create a new metrics layer using the provided registry.
    ///
    /// # Arguments
    /// - `registry`: A metrics registry used to create counters, gauges, and histograms.
    ///
    /// # Returns
    /// A new `HttpMetricsLayer` that can be applied to an Axum `Router` or `Service`.
    pub fn new(registry: Arc<dyn MetricsRegistry>) -> Self {
        Self { registry }
    }
}

/// Lightweight container placed into Request extensions so handlers/loggers can correlate logs with traces.
///
/// When the `otlp` feature is enabled, the current span's trace/span ids
/// are extracted and hex-encoded; otherwise both fields remain `None`.
#[derive(Clone, Debug)]
pub struct TraceContext {
    /// Hex-encoded OpenTelemetry trace id when available (32 hex chars).
    pub trace_id: Option<String>,
    /// Hex-encoded OpenTelemetry span id when available (16 hex chars).
    pub span_id: Option<String>,
}

/// Insert a `TraceContext` into the request's extensions.
///
/// This is a small helper so tests can exercise the insertion without
/// building the full service pipeline.
///
/// # Type Parameters
/// - `B`: Request body type.
///
/// # Arguments
/// - `req`: The request to mutate.
///
/// # Examples
/// ```rust,ignore
/// use axum::http::Request;
/// use commons::services::robotorq_service::middleware::insert_trace_context;
///
/// let mut req: Request<()> = Request::builder().uri("/x").body(()).unwrap();
/// insert_trace_context(&mut req);
/// ```
pub(crate) fn insert_trace_context<B>(req: &mut Request<B>) {
    // Default empty context. Make mutable only when OTLP feature is enabled
    // so we avoid an unused `mut` when the feature is disabled.
    #[cfg(feature = "otlp")]
    let mut tc = TraceContext {
        trace_id: None,
        span_id: None,
    };

    #[cfg(not(feature = "otlp"))]
    let tc = TraceContext {
        trace_id: None,
        span_id: None,
    };

    // If OTLP/tracing integration is available, extract ids from current span
    #[cfg(feature = "otlp")]
    {
        let span = Span::current();
        let cx = tracing_opentelemetry::OpenTelemetrySpanExt::context(&span);
        let sc = cx.span();
        let span_ctx = sc.span_context();

        // Convert trace/span ids to hex strings in a version-stable way using byte arrays.
        let trace_bytes = span_ctx.trace_id().to_bytes();
        if trace_bytes.iter().any(|&b| b != 0) {
            let tid_num = u128::from_be_bytes(trace_bytes);
            tc.trace_id = Some(format!("{:032x}", tid_num));
        }
        let span_bytes = span_ctx.span_id().to_bytes();
        if span_bytes.iter().any(|&b| b != 0) {
            let sid_num = u64::from_be_bytes(span_bytes);
            tc.span_id = Some(format!("{:016x}", sid_num));
        }
    }

    req.extensions_mut().insert(tc);
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let requests_total = self.registry.counter_vec(
            "http_requests_total",
            "Total HTTP requests",
            &["method", "status", "path"],
        );
        let inflight =
            self.registry
                .gauge("http_inflight_requests", "In-flight HTTP requests", &[]);
        let durations = self.registry.histogram_vec(
            "http_request_duration_seconds",
            "HTTP request durations in seconds",
            &["method", "status", "path"],
            Some(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
        );
        let errors_total = self.registry.counter_vec(
            "http_errors_total",
            "HTTP error responses (>=400)",
            &["method", "status_class", "path"],
        );

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
///
/// # Fields
/// - `inner`: Wrapped service handling the request.
/// - `requests_total`: Counter of total requests by method/status/path.
/// - `inflight`: Gauge tracking in-flight requests.
/// - `durations`: Histogram of request durations by method/status/path.
/// - `errors_total`: Counter of errors (status >= 400) by method/status_class/path.
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

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let start = Instant::now();
        self.inflight.inc();

        let method = req.method().as_str().to_string();
        // MatchedPath is set by axum Router; fallback to raw path if missing
        let path = req.extensions().get::<MatchedPath>().map_or_else(
            || req.uri().path().to_string(),
            |mp| mp.as_str().to_string(),
        );

        // Insert trace context into the request extensions so downstream handlers
        // and log emitters can access trace/span ids for correlation.
        insert_trace_context(&mut req);

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
                    let labels = [
                        ("method", method.as_str()),
                        ("status", status.as_str()),
                        ("path", path.as_str()),
                    ];
                    requests_total.inc(&labels);
                    durations.observe_duration(start.elapsed(), &labels);
                    if status_code >= 400 {
                        let class = if status_code >= 500 { "5xx" } else { "4xx" };
                        let elabels = [
                            ("method", method.as_str()),
                            ("status_class", class),
                            ("path", path.as_str()),
                        ];
                        errors_total.inc(&elabels);
                    }
                }
                Err(_err) => {
                    // On inner service error, count as 5xx and record duration
                    let labels = [
                        ("method", method.as_str()),
                        ("status", "500"),
                        ("path", path.as_str()),
                    ];
                    requests_total.inc(&labels);
                    durations.observe_duration(start.elapsed(), &labels);
                    let elabels = [
                        ("method", method.as_str()),
                        ("status_class", "5xx"),
                        ("path", path.as_str()),
                    ];
                    errors_total.inc(&elabels);
                }
            }
            inflight.dec();
            result
        })
    }
}
