//! Prometheus metrics integration helpers.
//!
//! Provides a shared `MetricsHandler` abstraction that owns a Prometheus
//! registry and convenience functions to register counters, gauges, and
//! histograms, plus export in text format.
use prometheus::{Registry, TextEncoder, Encoder, Counter, Gauge, Histogram, HistogramOpts};
use crate::util::schema::all_schema_versions;
use std::sync::Arc;
use crate::util::error::prometheus_error::PrometheusError;

/// Core metrics handler: owns the Prometheus registry and provides helpers.
/// 
/// # Fields
/// 
/// * `registry` - The Prometheus registry holding all registered metrics.
/// 
/// # Example
/// ```
/// use commons::util::metrics::MetricsHandler;
/// 
/// let handler = MetricsHandler::new();
/// // Now you can register metrics using the handler
/// let counter = handler.register_counter("example", "An example counter");
/// ```
#[derive(Clone)]
pub struct MetricsHandler {
    /// The Prometheus registry holding all registered metrics.
    pub registry: Registry,
}

impl MetricsHandler {
    /// Create a new handler with an empty registry.
    ///
    /// # Returns
    ///
    /// An `Arc<Self>` containing the new `MetricsHandler` instance.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// // Use the handler to register metrics
    /// ```
    pub fn new() -> Arc<Self> {
        Arc::new(Self { registry: Registry::new() })
    }

    /// Register a gauge per known schema type, set to its version.
    /// `prefix` should be a short identifier like "commons".
    ///
    /// # Fields
    ///
    /// * `prefix` - A short identifier for the metrics, e.g., "commons".
    ///
    /// # Returns
    ///
    /// A `Vec<Gauge>` containing the registered gauges for each schema type.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let gauges = handler.register_schema_version_gauges("commons");
    /// // Gauges are now registered and set to current schema versions
    /// ```
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
    ///
    /// # Fields
    ///
    /// * `prefix` - A short identifier for the metrics, e.g., "commons".
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<Gauge>` on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let gauges = handler.register_schema_version_gauges_result("commons")?;
    /// // Gauges are now registered and set to current schema versions
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
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

    /// Register a simple counter.
    ///
    /// # Fields
    ///
    /// * `name` - Name of the counter metric.
    /// * `help` - Description of the counter metric.
    ///
    /// # Returns
    ///
    /// A `Counter` instance that can be used to increment the counter value.
    ///
    /// # Panics
    ///
    /// Panics if the counter cannot be created (e.g., invalid metric name or help text).
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let counter = handler.register_counter("robots_total", "Total number of robots");
    /// // add a Robot to the Robot-Gateway
    /// counter.inc();
    /// // or increment by a specific amount
    /// counter.inc_by(5.0);
    /// ```
    pub fn register_counter(&self, name: &str, help: &str) -> Counter {
        let c = Counter::new(name, help).expect("counter");
        self.registry.register(Box::new(c.clone())).ok();
        c
    }

    /// Register a counter, returning errors on failure.
    ///
    /// # Fields
    ///
    /// * `name` - Name of the counter metric.
    /// * `help` - Description of the counter metric.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Counter` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let counter = handler.register_counter_result("robots_total", "Total number of robots")?;
    /// counter.inc();
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_counter_result(&self, name: &str, help: &str) -> Result<Counter, PrometheusError> {
        let c = Counter::new(name, help)?;
        self.registry.register(Box::new(c.clone()))?;
        Ok(c)
    }

    /// Register a simple gauge.
    /// 
    /// # Fields
    /// 
    /// * `name` - Name of the gauge metric.
    /// * `help` - Description of the gauge metric.
    /// 
    /// # Returns
    /// 
    /// A `Gauge` instance that can be used to set or update the gauge value.
    ///
    /// # Panics
    ///
    /// Panics if the gauge cannot be created (e.g., invalid metric name or help text).
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let gauge = handler.register_gauge("active_connections", "Number of active connections");
    /// // set current active connections
    /// gauge.set(10.0);
    /// // later, decrease connections
    /// gauge.set(7.0);
    /// ```
    pub fn register_gauge(&self, name: &str, help: &str) -> Gauge {
        let g = Gauge::new(name, help).expect("gauge");
        self.registry.register(Box::new(g.clone())).ok();
        g
    }

    /// Register a gauge, returning errors on failure.
    ///
    /// # Fields
    ///
    /// * `name` - Name of the gauge metric.
    /// * `help` - Description of the gauge metric.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Gauge` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let gauge = handler.register_gauge_result("active_connections", "Number of active connections")?;
    /// gauge.set(10.0);
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_gauge_result(&self, name: &str, help: &str) -> Result<Gauge, PrometheusError> {
        let g = Gauge::new(name, help)?;
        self.registry.register(Box::new(g.clone()))?;
        Ok(g)
    }

    /// Register a histogram with buckets.
    ///
    /// # Fields
    ///
    /// * `name` - Name of the histogram metric.
    /// * `help` - Description of the histogram metric.
    /// * `buckets` - Slice of bucket boundaries for the histogram.
    ///
    /// # Returns
    ///
    /// A `Histogram` instance that can be used to observe values.
    ///
    /// # Panics
    ///
    /// Panics if the histogram cannot be created (e.g., invalid metric name, help text, or bucket configuration).
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let buckets = [0.1, 1.0, 10.0];
    /// let hist = handler.register_histogram("request_duration", "Request duration in seconds", &buckets);
    /// hist.observe(0.5);
    /// hist.observe(2.0);
    /// ```
    pub fn register_histogram(&self, name: &str, help: &str, buckets: &[f64]) -> Histogram {
        let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec())).expect("histogram");
        self.registry.register(Box::new(h.clone())).ok();
        h
    }

    /// Register a histogram, returning errors on failure.
    ///
    /// # Fields
    ///
    /// * `name` - Name of the histogram metric.
    /// * `help` - Description of the histogram metric.
    /// * `buckets` - Slice of bucket boundaries for the histogram.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Histogram` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let buckets = [0.1, 1.0, 10.0];
    /// let hist = handler.register_histogram_result("request_duration", "Request duration in seconds", &buckets)?;
    /// hist.observe(0.5);
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_histogram_result(&self, name: &str, help: &str, buckets: &[f64]) -> Result<Histogram, PrometheusError> {
        let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec()))?;
        self.registry.register(Box::new(h.clone()))?;
        Ok(h)
    }

    /// Register an error counter (convenience for common use).
    ///
    /// This is equivalent to `register_counter(&format!("{}_errors_total", name), help)`.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the error counter metric.
    /// * `help` - Description of the error counter metric.
    ///
    /// # Returns
    ///
    /// A `Counter` instance for tracking errors.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let err_counter = handler.register_error_counter("http", "HTTP request errors");
    /// // on error
    /// err_counter.inc();
    /// ```
    pub fn register_error_counter(&self, name: &str, help: &str) -> Counter {
        self.register_counter(&format!("{}_errors_total", name), help)
    }

    /// Error counter variant returning Result.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the error counter metric.
    /// * `help` - Description of the error counter metric.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Counter` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let err_counter = handler.register_error_counter_result("http", "HTTP request errors")?;
    /// err_counter.inc();
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_error_counter_result(&self, name: &str, help: &str) -> Result<Counter, PrometheusError> {
        self.register_counter_result(&format!("{}_errors_total", name), help)
    }

    /// Register a batch size gauge (convenience for batch processing metrics).
    ///
    /// This is equivalent to `register_gauge(&format!("{}_batch_size", name), help)`.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the batch size gauge metric.
    /// * `help` - Description of the batch size gauge metric.
    ///
    /// # Returns
    ///
    /// A `Gauge` instance for tracking batch sizes.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let batch_gauge = handler.register_batch_size_gauge("processor", "Current batch size");
    /// batch_gauge.set(50.0);
    /// // later
    /// batch_gauge.set(30.0);
    /// ```
    pub fn register_batch_size_gauge(&self, name: &str, help: &str) -> Gauge {
        self.register_gauge(&format!("{}_batch_size", name), help)
    }

    /// Batch size gauge variant returning Result.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the batch size gauge metric.
    /// * `help` - Description of the batch size gauge metric.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Gauge` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let batch_gauge = handler.register_batch_size_gauge_result("processor", "Current batch size")?;
    /// batch_gauge.set(50.0);
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_batch_size_gauge_result(&self, name: &str, help: &str) -> Result<Gauge, PrometheusError> {
        self.register_gauge_result(&format!("{}_batch_size", name), help)
    }

    /// Register a processing time histogram with default buckets (convenience).
    ///
    /// Uses default buckets: [0.001, 0.01, 0.1, 1.0, 10.0] seconds.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the processing time histogram metric.
    /// * `help` - Description of the processing time histogram metric.
    ///
    /// # Returns
    ///
    /// A `Histogram` instance for tracking processing times.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let proc_hist = handler.register_processing_time_histogram("task", "Task processing time");
    /// proc_hist.observe(0.05);
    /// proc_hist.observe(0.2);
    /// ```
    pub fn register_processing_time_histogram(&self, name: &str, help: &str) -> Histogram {
        let buckets = [0.001, 0.01, 0.1, 1.0, 10.0]; // seconds
        self.register_histogram(&format!("{}_processing_time_seconds", name), help, &buckets)
    }

    /// Processing time histogram variant returning Result.
    ///
    /// # Fields
    ///
    /// * `name` - Base name for the processing time histogram metric.
    /// * `help` - Description of the processing time histogram metric.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Histogram` instance on success, or a `PrometheusError` on failure.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// let proc_hist = handler.register_processing_time_histogram_result("task", "Task processing time")?;
    /// proc_hist.observe(0.05);
    /// # Ok::<(), commons::util::error::prometheus_error::PrometheusError>(())
    /// ```
    pub fn register_processing_time_histogram_result(&self, name: &str, help: &str) -> Result<Histogram, PrometheusError> {
        let buckets = [0.001, 0.01, 0.1, 1.0, 10.0]; // seconds
        self.register_histogram_result(&format!("{}_processing_time_seconds", name), help, &buckets)
    }

    /// Export current metrics in Prometheus text format.
    ///
    /// # Returns
    ///
    /// A `String` containing the metrics in Prometheus text format.
    ///
    /// # Panics
    ///
    /// Panics if the metrics cannot be encoded to Prometheus text format.
    ///
    /// # Example
    /// ```
    /// use commons::util::metrics::MetricsHandler;
    ///
    /// let handler = MetricsHandler::new();
    /// // Register some metrics...
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

    /// Test that the handler can register various types of metrics and export them successfully.
    ///
    /// This test verifies the integration of counter, gauge, histogram, error counter,
    /// batch size gauge, and processing time histogram registration, along with
    /// basic metric operations and text export functionality.
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

    /// Test that schema version gauges are properly registered and present in exported metrics.
    ///
    /// This test ensures that calling `register_schema_version_gauges` with a prefix
    /// results in the expected gauge metrics appearing in the exported Prometheus text format.
    #[test]
    fn schema_version_gauges_present_in_export() {
        let h = MetricsHandler::new();
        h.register_schema_version_gauges("commons");
        let text = h.export_text();
        assert!(text.contains("commons_schema_version_token") || text.contains("commons_schema_version_robot"));
    }
}
