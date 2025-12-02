//! Service-level metrics context providing standardized counters and gauges.
//!
//! This context is constructed when a metrics registry is configured for the
//! HTTP server and passed to services via `RoboTorqServiceMetricsExt::set_metrics_context`.
//! It centralizes lifecycle metrics and label conventions so individual services
//! do not need to duplicate boilerplate registration logic.
//!
//! # Metrics Registered
//!
//! - `service_starts_total`     (counter)
//! - `service_stops_total`      (counter)
//! - `service_errors_total`     (counter)
//! - `service_health_checks_total` (counter)
//! - `service_ready`            (gauge: 0/1 readiness)
//! - `service_uptime_seconds`   (gauge: monotonic uptime)
//!
//! # Example
//! ```rust
//! use std::sync::Arc;
//! use commons::services::robotorq_service::service_metrics_context::ServiceMetricsContext;
//! use commons::util::metrics::{PrometheusRegistry, MetricsRegistry};
//!
//! let registry: Arc<dyn MetricsRegistry> = Arc::new(PrometheusRegistry::new("sandbox", "http", "dev"));
//! let ctx = ServiceMetricsContext::new(registry, commons::services::robotorq_service::label_source::LabelSet {
//!     service: "sandbox".to_string(),
//!     component: "http".to_string(),
//!     version: "dev".to_string(),
//!     subject: "core".to_string(),
//! });
//! ctx.mark_start();
//! ctx.inc_health();
//! ctx.set_ready(true);
//! ctx.refresh_uptime();
//! ```

use super::label_source::LabelSet;
use crate::util::metrics::{MetricCounter, MetricGauge, MetricsRegistry};
use std::sync::Arc;
use std::time::Instant;

/// Shared service-level metrics with common labels and lifecycle counters.
pub struct ServiceMetricsContext {
    registry: Arc<dyn MetricsRegistry>,
    labels: LabelSet,
    start_instant: Instant,
    starts_total: Box<dyn MetricCounter + Send + Sync>,
    stops_total: Box<dyn MetricCounter + Send + Sync>,
    errors_total: Box<dyn MetricCounter + Send + Sync>,
    health_checks_total: Box<dyn MetricCounter + Send + Sync>,
    ready_gauge: Box<dyn MetricGauge + Send + Sync>,
    uptime_seconds: Box<dyn MetricGauge + Send + Sync>,
}

impl ServiceMetricsContext {
    /// Create a new service metrics context bound to the provided registry and labels.
    pub fn new(registry: Arc<dyn MetricsRegistry>, labels: LabelSet) -> Self {
        let starts_total = registry.counter("service_starts_total", "Total service starts", &[]);
        let stops_total = registry.counter("service_stops_total", "Total service stops", &[]);
        let errors_total = registry.counter("service_errors_total", "Total service errors", &[]);
        let health_checks_total = registry.counter(
            "service_health_checks_total",
            "Total health check evaluations",
            &[],
        );
        let ready_gauge = registry.gauge("service_ready", "Service readiness flag", &[]);
        let uptime_seconds =
            registry.gauge("service_uptime_seconds", "Service uptime seconds", &[]);
        Self {
            registry,
            labels,
            start_instant: Instant::now(),
            starts_total,
            stops_total,
            errors_total,
            health_checks_total,
            ready_gauge,
            uptime_seconds,
        }
    }

    /// Increment the total starts counter.
    pub fn mark_start(&self) {
        self.starts_total.inc();
    }
    /// Increment the total stops counter.
    pub fn mark_stop(&self) {
        self.stops_total.inc();
    }
    /// Increment the total health checks counter.
    pub fn inc_health(&self) {
        self.health_checks_total.inc();
    }
    /// Increment the total errors counter.
    pub fn inc_error(&self) {
        self.errors_total.inc();
    }
    /// Set readiness gauge to 1 when ready, 0 otherwise.
    pub fn set_ready(&self, ready: bool) {
        self.ready_gauge.set(if ready { 1.0 } else { 0.0 });
    }
    /// Update the uptime gauge from the start instant.
    pub fn refresh_uptime(&self) {
        let secs = self.start_instant.elapsed().as_secs_f64();
        self.uptime_seconds.set(secs);
    }

    /// Access the underlying metrics registry.
    pub fn registry(&self) -> &Arc<dyn MetricsRegistry> {
        &self.registry
    }
    /// The service label applied to emitted metrics.
    pub fn service(&self) -> &str {
        &self.labels.service
    }
    /// The component label applied to emitted metrics.
    pub fn component(&self) -> &str {
        &self.labels.component
    }
    /// The version label applied to emitted metrics.
    pub fn version(&self) -> &str {
        &self.labels.version
    }
    /// The subject label applied to emitted metrics.
    pub fn subject(&self) -> &str {
        &self.labels.subject
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::robotorq_service::label_source::build_label_set;
    use crate::util::metrics::{MetricsRegistry, PrometheusRegistry};

    #[test]
    fn context_registers_and_updates() {
        let registry: Arc<dyn MetricsRegistry> =
            Arc::new(PrometheusRegistry::new("svc", "http", "dev"));
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        let labels = build_label_set(&cfg);
        let ctx = ServiceMetricsContext::new(registry.clone(), labels);
        ctx.mark_start();
        ctx.inc_health();
        ctx.set_ready(true);
        ctx.refresh_uptime();
        let text = registry.export_text();
        assert!(text.contains("service_starts_total"));
        assert!(text.contains("service_health_checks_total"));
        assert!(text.contains("service_ready"));
        assert!(text.contains("service_uptime_seconds"));
    }

    #[test]
    fn ready_and_labels_exposed() {
        let registry: Arc<dyn MetricsRegistry> =
            Arc::new(PrometheusRegistry::new("svc2", "http", "dev2"));
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        let labels = build_label_set(&cfg);
        let ctx = ServiceMetricsContext::new(registry.clone(), labels);
        ctx.set_ready(false);
        // service_ready should be present as a gauge with value 0.0
        let text = registry.export_text();
        assert!(text.contains("service_ready"));
        assert_eq!(ctx.component(), "http");
    }
}
