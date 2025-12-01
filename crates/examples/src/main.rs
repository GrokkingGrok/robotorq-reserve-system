//! Sandbox playground that wires a small ``RoboTorqService`` implementation
//! over the common HTTP helpers so we can spin it up on demand.
//!
//! Set `ROBOTORQ_SANDBOX_PORT` to override the default TCP port (9000) when
//! running interactively.

use std::sync::Arc;

use commons::{
    services::http::{HttpServer, HttpServerConfig, label_source::build_label_set},
    util::config::load_robotorq_config,
    util::error::InvariantError,
    util::metrics::{MetricsRegistry, PrometheusRegistry},
};
use tokio::sync::Mutex;

const DEFAULT_PORT: u16 = 9000;

mod sandbox;
use crate::sandbox::SandboxService;

#[tokio::main]
async fn main() -> Result<(), InvariantError> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Read port override from env, default to 9000
    let port = std::env::var("ROBOTORQ_SANDBOX_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    // Load config to derive labels for metrics
    let config = load_robotorq_config(None).map_err(commons::util::error::config_error::ConfigError::Invalid)?;
    let labels = build_label_set(&config);

    // Build HTTP server config using local defaults and override port.
    // Metrics registry will be auto-created by HttpServer using config-derived labels.
    let http_config = HttpServerConfig::local_defaults(port);

    // Construct service and wrap for HttpServer
    let service = Arc::new(Mutex::new(SandboxService::new(http_config.clone(), &labels)));

    // Create server and use commons-side lifecycle with autoload initialization
    let server = HttpServer::new(service, http_config);
    server.start_autoload().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::services::http::{HttpEndpoint, HttpService, RoboTorqService};
    use commons::util::config::load_robotorq_config;
    use commons::services::http::label_source::build_label_set;

    #[test]
    fn health_counter_increments() {
        let test_config = HttpServerConfig {
            service: HttpService::new("127.0.0.1", 0),
            health: HttpEndpoint::new("/health"),
            metrics: HttpEndpoint::new("/metrics"),
            timeout_seconds: Some(30),
            max_body_size_bytes: Some(1024 * 1024),
            cors_permissive: false,
            metrics_registry: None,
        };
        // Create test labels
        let test_labels = commons::services::http::label_source::LabelSet {
            service: "test".to_string(),
            component: "http".to_string(),
            version: "0.0.1".to_string(),
            subject: "core".to_string(),
        };
        let service = SandboxService::new(test_config, &test_labels);
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
