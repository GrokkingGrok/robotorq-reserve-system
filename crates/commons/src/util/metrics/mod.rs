use prometheus::{Registry, TextEncoder, Encoder, Counter, Gauge, Histogram, HistogramOpts};
use crate::util::schema::all_schema_versions;
use std::sync::Arc;
use crate::util::error::prometheus_error::PrometheusError;

/// Core metrics handler: owns the Prometheus registry and provides helpers.
#[derive(Clone)]
pub struct MetricsHandler {
    pub registry: Registry,
}

impl MetricsHandler {
    /// Create a new handler with an empty registry.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { registry: Registry::new() })
    }

    /// Register a gauge per known schema type, set to its version.
    /// `prefix` should be a short identifier like "commons".
    pub fn register_schema_version_gauges(&self, prefix: &str) -> Vec<Gauge> {
        fn to_snake(name: &str) -> String {
            let mut out = String::with_capacity(name.len() * 2);
            for (i, ch) in name.chars().enumerate() {
                if ch.is_uppercase() {
                    if i > 0 { out.push('_'); }
                    out.push(ch.to_ascii_lowercase());
                } else {
                    out.push(ch);
                }
            }
            out
        }

        let mut gauges = Vec::new();
        for (type_name, version) in all_schema_versions() {
            let metric_name = format!("{}_schema_version_{}", prefix, to_snake(type_name));
            let g = self.register_gauge(&metric_name, "schema version of serialized type");
            g.set(version as f64);
            gauges.push(g);
        }
        gauges
    }

    /// Like `register_schema_version_gauges` but returns errors on registration failures.
    pub fn register_schema_version_gauges_result(&self, prefix: &str) -> Result<Vec<Gauge>, PrometheusError> {
        fn to_snake(name: &str) -> String {
            let mut out = String::with_capacity(name.len() * 2);
            for (i, ch) in name.chars().enumerate() {
                if ch.is_uppercase() {
                    if i > 0 { out.push('_'); }
                    out.push(ch.to_ascii_lowercase());
                } else {
                    out.push(ch);
                }
            }
            out
        }

        let mut gauges = Vec::new();
        for (type_name, version) in all_schema_versions() {
            let metric_name = format!("{}_schema_version_{}", prefix, to_snake(type_name));
            let g = self.register_gauge_result(&metric_name, "schema version of serialized type")?;
            g.set(version as f64);
            gauges.push(g);
        }
        Ok(gauges)
    }

    /// Register a counter.
    pub fn register_counter(&self, name: &str, help: &str) -> Counter {
        let c = Counter::new(name, help).expect("counter");
        self.registry.register(Box::new(c.clone())).ok();
        c
    }

    /// Register a counter, returning errors on failure.
    pub fn register_counter_result(&self, name: &str, help: &str) -> Result<Counter, PrometheusError> {
        let c = Counter::new(name, help)?;
        self.registry.register(Box::new(c.clone()))?;
        Ok(c)
    }

    /// Register a gauge.
    pub fn register_gauge(&self, name: &str, help: &str) -> Gauge {
        let g = Gauge::new(name, help).expect("gauge");
        self.registry.register(Box::new(g.clone())).ok();
        g
    }

    /// Register a gauge, returning errors on failure.
    pub fn register_gauge_result(&self, name: &str, help: &str) -> Result<Gauge, PrometheusError> {
        let g = Gauge::new(name, help)?;
        self.registry.register(Box::new(g.clone()))?;
        Ok(g)
    }

    /// Register a histogram with buckets.
    pub fn register_histogram(&self, name: &str, help: &str, buckets: &[f64]) -> Histogram {
        let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec())).expect("histogram");
        self.registry.register(Box::new(h.clone())).ok();
        h
    }

    /// Register a histogram, returning errors on failure.
    pub fn register_histogram_result(&self, name: &str, help: &str, buckets: &[f64]) -> Result<Histogram, PrometheusError> {
        let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec()))?;
        self.registry.register(Box::new(h.clone()))?;
        Ok(h)
    }

    /// Register an error counter (convenience for common use).
    pub fn register_error_counter(&self, name: &str, help: &str) -> Counter {
        self.register_counter(&format!("{}_errors_total", name), help)
    }

    /// Error counter variant returning Result.
    pub fn register_error_counter_result(&self, name: &str, help: &str) -> Result<Counter, PrometheusError> {
        self.register_counter_result(&format!("{}_errors_total", name), help)
    }

    /// Register a batch size gauge (convenience for batch processing metrics).
    pub fn register_batch_size_gauge(&self, name: &str, help: &str) -> Gauge {
        self.register_gauge(&format!("{}_batch_size", name), help)
    }

    /// Batch size gauge variant returning Result.
    pub fn register_batch_size_gauge_result(&self, name: &str, help: &str) -> Result<Gauge, PrometheusError> {
        self.register_gauge_result(&format!("{}_batch_size", name), help)
    }

    /// Register a processing time histogram with default buckets (convenience).
    pub fn register_processing_time_histogram(&self, name: &str, help: &str) -> Histogram {
        let buckets = [0.001, 0.01, 0.1, 1.0, 10.0]; // seconds
        self.register_histogram(&format!("{}_processing_time_seconds", name), help, &buckets)
    }

    /// Processing time histogram variant returning Result.
    pub fn register_processing_time_histogram_result(&self, name: &str, help: &str) -> Result<Histogram, PrometheusError> {
        let buckets = [0.001, 0.01, 0.1, 1.0, 10.0]; // seconds
        self.register_histogram_result(&format!("{}_processing_time_seconds", name), help, &buckets)
    }

    /// Export current metrics in Prometheus text format.
    ///
    /// Example usage:
    /// ```rust,ignore
    /// use commons::util::metrics::MetricsHandler;
    /// use commons::robot_gateway_metrics::RobotGatewayMetrics;
    /// use commons::{RobotId};
    ///
    /// let handler = MetricsHandler::new();
    /// // Optional: expose schema version gauges for observability
    /// handler.register_schema_version_gauges("commons");
    /// // Service-specific metrics (e.g., RobotGateway)
    /// let gw_metrics = RobotGatewayMetrics::new(&handler, "robot_gateway");
    /// let gw = commons::robot_gateway::RobotGateway::single(RobotId::new()).with_metrics(gw_metrics);
    /// // ... perform operations that update metrics ...
    /// let text = handler.export_text();
    /// // Serve `text` via HTTP endpoint or write to logs
    /// ```
    pub fn export_text(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).expect("encode");
        String::from_utf8(buffer).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_registers_and_exports() {
        let h = MetricsHandler::new();
        let c = h.register_counter("test_counter", "desc");
        let g = h.register_gauge("test_gauge", "desc");
        let buckets = [0.01, 0.1, 1.0];
        let hist = h.register_histogram("test_hist", "desc", &buckets);
        let err_counter = h.register_error_counter("test", "error counter");
        let batch_gauge = h.register_batch_size_gauge("test", "batch size");
        let proc_hist = h.register_processing_time_histogram("test", "processing time");
        c.inc_by(2.0);
        g.set(1.0);
        hist.observe(0.05);
        err_counter.inc();
        batch_gauge.set(10.0);
        proc_hist.observe(0.5);
        let _text = h.export_text();
    }

    #[test]
    fn schema_version_gauges_present_in_export() {
        let h = MetricsHandler::new();
        h.register_schema_version_gauges("commons");
        let text = h.export_text();
        assert!(text.contains("commons_schema_version_token") || text.contains("commons_schema_version_robot"));
    }
}
