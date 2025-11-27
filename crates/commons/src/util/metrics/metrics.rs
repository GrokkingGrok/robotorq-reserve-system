use prometheus::{Registry, TextEncoder, Encoder, Counter, Gauge, Histogram, HistogramOpts};
use std::sync::Arc;

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

    /// Register a counter.
    pub fn register_counter(&self, name: &str, help: &str) -> Counter {
        let c = Counter::new(name, help).expect("counter");
        self.registry.register(Box::new(c.clone())).ok();
        c
    }

    /// Register a gauge.
    pub fn register_gauge(&self, name: &str, help: &str) -> Gauge {
        let g = Gauge::new(name, help).expect("gauge");
        self.registry.register(Box::new(g.clone())).ok();
        g
    }

    /// Register a histogram with buckets.
    pub fn register_histogram(&self, name: &str, help: &str, buckets: &[f64]) -> Histogram {
        let h = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets.to_vec())).expect("histogram");
        self.registry.register(Box::new(h.clone())).ok();
        h
    }

    /// Export current metrics in Prometheus text format.
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
        c.inc_by(2.0);
        g.set(1.0);
        hist.observe(0.05);
        let _text = h.export_text();
    }
}
