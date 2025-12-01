//! Sandbox playground that wires a small ``RoboTorqService`` implementation
//! over the common HTTP helpers so we can spin it up on demand.
//!
//! Set `ROBOTORQ_SANDBOX_PORT` to override the default TCP port (9000) when
//! running interactively.

use std::{sync::Arc, time::Instant};

use commons::{
    services::http::{HttpEndpoint, HttpServer, HttpServerConfig, HttpService, RoboTorqService},
    util::{
        config::{HttpConfig, load_robotorq_config},
        error::{InvariantError, config_error::ConfigError},
        metrics::{MetricCounter, MetricsRegistry, PrometheusRegistry},
    },
};
use tracing::info;
use tracing_subscriber;

const DEFAULT_PORT: u16 = 9000;

/// Lightweight service used for playground demos.
pub struct SandboxService {
    metrics: Arc<PrometheusRegistry>,
    health_checks: Box<dyn MetricCounter + Send + Sync>,
    start_time: Instant,
}

impl SandboxService {
    /// Build the sandbox service and register the health counter.
    pub fn new() -> Self {
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
}

#[tokio::main]
async fn main() -> Result<(), InvariantError> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let port = std::env::var("ROBOTORQ_SANDBOX_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    // Load the shared roboTorq configuration so we can experiment with real settings.
    let config = load_robotorq_config(None).map_err(ConfigError::Invalid)?;
    info!(
        schema_version = config.schema_version,
        mode = ?config.mode,
        http_address = %config.http.address,
        http_port = config.http.port,
        "loaded RoboTorq configuration"
    );

    // Build the HTTP server config from the loaded settings, honoring the sandbox override.
    let http_config = build_http_server_config(&config.http, Some(port));

    let service = Arc::new(SandboxService::new());
    info!(
        port = http_config.service.port,
        "starting the RoboTorq sandbox"
    );
    HttpServer::new(service, http_config).start().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_counter_increments() {
        let service = SandboxService::new();
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
}

/// Translate the config’s HTTP section into the HTTP server wiring we expose.
/// This keeps the sandbox in sync with any defaults or overrides defined in `robotorq.toml`.
fn build_http_server_config(http_cfg: &HttpConfig, override_port: Option<u16>) -> HttpServerConfig {
    let port = override_port.unwrap_or(http_cfg.port);
    HttpServerConfig {
        service: HttpService::new(&http_cfg.address, port),
        health: HttpEndpoint::new(&http_cfg.health.path),
        metrics: HttpEndpoint::new(&http_cfg.metrics.path),
        timeout_seconds: http_cfg.request_timeout_seconds,
        max_body_size_bytes: http_cfg.max_body_size_bytes,
        cors_permissive: http_cfg.cors.enabled,
    }
}
