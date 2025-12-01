//! Sandbox service module
//!
//! Holds the `SandboxService` used by the examples binary so that
//! `main.rs` stays lean. This module is private to the examples crate.

use std::{sync::Arc, time::Instant};

use commons::{
    services::http::{HttpServerConfig, RoboTorqService},
    util::error::InvariantError,
    util::metrics::{MetricCounter, MetricsRegistry, PrometheusRegistry},
};

/// Lightweight service used for playground demos.
pub struct SandboxService {
    metrics: Arc<PrometheusRegistry>,
    health_checks: Box<dyn MetricCounter + Send + Sync>,
    start_time: Instant,
    #[allow(dead_code)]
    http_config: HttpServerConfig,
}

impl SandboxService {
    /// Build the sandbox service and register the health counter.
    pub fn new(http_config: HttpServerConfig) -> Self {
        let metrics = Arc::new(PrometheusRegistry::new("sandbox", "playground", "dev"));
        let health_checks = metrics.counter(
            "sandbox_health_checks_total",
            "Number of sandbox health checks",
            &[],
        );
        Self {
            metrics,
            health_checks,
            start_time: Instant::now(),
            http_config,
        }
    }
}

impl RoboTorqService for SandboxService {
    fn health_check(&self) -> Result<String, InvariantError> {
        self.health_checks.inc();
        Ok(format!(
            "Sandbox healthy (uptime {:.2}s)",
            self.start_time.elapsed().as_secs_f64(),
        ))
    }

    fn export_metrics(&self) -> String {
        self.metrics.export_text()
    }

    fn start(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
        async { Ok(()) }
    }
}
