//! Sandbox service module
//!
//! Holds the `SandboxService` used by the examples binary so that
//! `main.rs` stays lean. This module is private to the examples crate.

use std::{sync::Arc, time::Instant};

use commons::{
    services::robotorq_service::RoboTorqService,
    services::robotorq_service::label_source::LabelSet,
    util::error::ServiceError,
    util::metrics::{MetricCounter, MetricsRegistry},
};

/// Lightweight service used for playground demos.
pub struct SandboxService {
    registry: Arc<dyn MetricsRegistry>,
    health_checks: Box<dyn MetricCounter + Send + Sync>,
    start_time: Instant,
}

impl SandboxService {
    /// Build the sandbox service using an injected shared registry.
    pub fn new(_labels: &LabelSet, registry: Arc<dyn MetricsRegistry>) -> Self {
        let health_checks = registry.counter(
            "sandbox_health_checks_total",
            "Number of sandbox health checks",
            &[],
        );
        Self {
            registry,
            health_checks,
            start_time: Instant::now(),
        }
    }
}

impl RoboTorqService for SandboxService {
    fn health_check(&self) -> Result<String, ServiceError> {
        self.health_checks.inc();
        Ok(format!(
            "Sandbox healthy (uptime {:.2}s)",
            self.start_time.elapsed().as_secs_f64(),
        ))
    }

    fn export_metrics(&self) -> String {
        self.registry.export_text()
    }

    async fn start(&self) -> Result<(), ServiceError> {
        Ok(())
    }
}

impl commons::services::robotorq_service::ServiceLifecycle for SandboxService {}

impl commons::services::robotorq_service::HealthContributor for SandboxService {
    fn health_status(&self) -> String {
        self.health_check()
            .unwrap_or_else(|e| format!("error: {:?}", e))
    }
}

impl commons::services::robotorq_service::MetricsContributor for SandboxService {
    fn set_metrics_context(
        &mut self,
        _ctx: Option<
            std::sync::Arc<
                commons::services::robotorq_service::service_metrics_context::ServiceMetricsContext,
            >,
        >,
    ) {
    }
    fn export_metrics(&self) -> String {
        <SandboxService as commons::services::robotorq_service::RoboTorqService>::export_metrics(
            self,
        )
    }
}
