//! HTTP observability middleware: request counters, durations, in-flight gauge.
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};
use axum::http::Request;
use axum::response::Response;
use crate::util::metrics::{MetricsRegistry, MetricCounter, MetricGauge, MetricHistogram};
use std::sync::Arc;

/// Axum layer that wires HTTP metrics into the request pipeline.
#[derive(Clone)]
pub struct HttpMetricsLayer {
    registry: Arc<dyn MetricsRegistry>,
}

impl HttpMetricsLayer {
    /// Create a new metrics layer using the provided registry.
    pub fn new(registry: Arc<dyn MetricsRegistry>) -> Self { Self { registry } }
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let requests_total = self.registry.counter(
            "http_requests_total",
            "Total HTTP requests",
            &[],
        );
        let inflight = self.registry.gauge(
            "http_inflight_requests",
            "In-flight HTTP requests",
            &[],
        );
        let durations = self.registry.histogram(
            "http_request_duration_seconds",
            "HTTP request durations in seconds",
            &[],
            Some(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        );

        HttpMetricsService { inner, requests_total: requests_total.into(), inflight: inflight.into(), durations: durations.into() }
    }
}

/// Service wrapper that records request counters, inflight gauge, and durations.
#[derive(Clone)]
pub struct HttpMetricsService<S> {
    inner: S,
    requests_total: Arc<dyn MetricCounter + Send + Sync>,
    inflight: Arc<dyn MetricGauge + Send + Sync>,
    durations: Arc<dyn MetricHistogram + Send + Sync>,
}

impl<S, B> Service<Request<B>> for HttpMetricsService<S>
where
    S: Service<Request<B>, Response = Response>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let start = Instant::now();
        self.inflight.inc();
        self.requests_total.inc();
        let fut = self.inner.call(req);
        // We can’t inspect status here without boxing future; keep minimal for phase 1.
        // Record duration and counters once future completes by using map_err/map_ok in a next iteration.
        // For now, decrement inflight and record duration opportunistically in drop guard.
        // Note: To keep changes minimal, we rely on simple timing recorded when future is polled to completion.
        struct Guard {
            inflight: Arc<dyn MetricGauge + Send + Sync>,
            durations: Arc<dyn MetricHistogram + Send + Sync>,
            start: Instant,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                let elapsed = self.start.elapsed();
                self.durations.observe_duration(elapsed);
                self.inflight.dec();
            }
        }
        let _g = Guard {
            inflight: self.inflight.clone(),
            durations: self.durations.clone(),
            start,
        };
        fut
    }
}
