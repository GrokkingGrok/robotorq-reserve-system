//! Sandbox playground that wires a small ``RoboTorqService`` implementation
//! over the common HTTP helpers so we can spin it up on demand.
//!
//! Set `ROBOTORQ_SANDBOX_PORT` to override the default TCP port (9000) when
//! running interactively.

use std::sync::Arc;

use commons::{
    services::robotorq_service::{HttpServerBuilder, label_source::build_label_set},
    util::config::load_robotorq_config,
    util::error::ExampleError,
    util::metrics::PrometheusRegistry,
};
use tokio::sync::Mutex;

const DEFAULT_PORT: u16 = 9000;

mod sandbox;
use crate::sandbox::SandboxService;

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Read port override from env, default to 9000
    let port = std::env::var("ROBOTORQ_SANDBOX_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    // Load config to derive labels for metrics
    let config =
        load_robotorq_config(None).map_err(|e| ExampleError::Other(format!("config load: {e}")))?;
    let labels = build_label_set(&config);

    // Construct shared registry with config-derived labels for unified metrics.
    let registry: Arc<dyn commons::util::metrics::MetricsRegistry> = Arc::new(
        PrometheusRegistry::new(&labels.service, &labels.component, &labels.version),
    );

    // Build service using injected registry (no internal registry duplication).
    let service = Arc::new(Mutex::new(SandboxService::new(
        &labels,
        Arc::clone(&registry),
    )));

    // Use builder pattern for HTTP server wiring.
    HttpServerBuilder::new(service)
        .with_port(port)
        .with_registry(Arc::clone(&registry))
        .with_static_labels(
            &labels.service,
            &labels.component,
            &labels.version,
            &labels.subject,
        )
        .build_and_start_autoload()
        .await
        .map_err(|e| ExampleError::Other(format!("service failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::services::robotorq_service::label_source::build_label_set;
    use commons::util::config::load_robotorq_config;
    use commons::util::metrics::PrometheusRegistry;

    #[test]
    fn health_counter_increments() {
        // Create test labels
        let test_labels = commons::services::robotorq_service::label_source::LabelSet {
            service: "test".to_string(),
            component: "http".to_string(),
            version: "0.0.1".to_string(),
            subject: "core".to_string(),
        };
        let registry: Arc<dyn commons::util::metrics::MetricsRegistry> =
            Arc::new(PrometheusRegistry::new(
                &test_labels.service,
                &test_labels.component,
                &test_labels.version,
            ));
        let service = SandboxService::new(&test_labels, Arc::clone(&registry));
        fn extract_metric_value(metrics: &str, name: &str) -> Option<f64> {
            metrics
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        return None;
                    }
                    let mut parts = trimmed.split_whitespace();
                    let metric = parts.next()?;
                    if !metric.starts_with(name) {
                        return None;
                    }
                    parts.next()?.parse().ok()
                })
                .next()
        }

        // Bring trait into scope for method resolution
        use commons::services::robotorq_service::RoboTorqService;
        service.health_check().unwrap();
        let first = extract_metric_value(&service.export_metrics(), "sandbox_health_checks_total");
        assert_eq!(first, Some(1.0));
        service.health_check().unwrap();
        let value = extract_metric_value(&service.export_metrics(), "sandbox_health_checks_total");
        assert_eq!(value, Some(2.0));
    }

    #[test]
    fn test_config_derived_labels() {
        // Load config and build labels
        let config = load_robotorq_config(None).expect("config should load");
        let labels = build_label_set(&config);

        // Assert labels are derived (not hardcoded)
        assert!(!labels.service.is_empty());
        assert_eq!(labels.component, "http");
        assert!(!labels.version.is_empty());
        assert!(labels.subject == "core" || labels.subject == "sim");
    }
}
