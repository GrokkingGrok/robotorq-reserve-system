use prometheus::{Counter, Gauge};
use commons::util::metrics::MetricsHandler;
use std::sync::Arc;

pub struct RobotGatewayMetrics {
    handler: Arc<MetricsHandler>,
    registered_robots: Gauge,
    batches_captured_total: Counter,
    batches_rejected_total: Counter,
}

impl RobotGatewayMetrics {
    /// Create a new metrics set for the robot gateway.
    /// Internally allocates a fresh `MetricsHandler` and registers schema gauges plus
    /// gateway-specific counters/gauges.
    #[must_use]
    pub fn new(prefix: &str) -> Self {
        let handler = MetricsHandler::new();
        handler.register_schema_version_gauges("commons");
        let registered_robots = handler.register_gauge(
            &format!("{}_registered_robots", prefix),
            "number of robots registered in gateway",
        );
        let batches_captured_total = handler.register_counter(
            &format!("{}_batches_captured_total", prefix),
            "total unmapped ore batches successfully captured",
        );
        let batches_rejected_total = handler.register_counter(
            &format!("{}_batches_rejected_total", prefix),
            "total batch capture attempts rejected (e.g., unknown robot)",
        );
        Self { handler, registered_robots, batches_captured_total, batches_rejected_total }
    }

    pub fn set_registered(&self, n: usize) { 
        #[allow(clippy::as_conversions)]
        self.registered_robots.set(n as f64); 
    }
    pub fn inc_captured(&self) { self.batches_captured_total.inc(); }
    pub fn inc_rejected(&self) { self.batches_rejected_total.inc(); }

    /// Getter returning a cloned Arc to the underlying handler for server/export usage.
    #[must_use]
    pub fn get_handler(&self) -> Arc<MetricsHandler> { Arc::clone(&self.handler) }
}