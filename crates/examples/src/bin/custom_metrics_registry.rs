//! Rich integration example demonstrating metrics decoupling and lifecycle hooks.
//!
//! Showcases Option C (kept as a full integration sample) with:
//! - Builder pattern (`HttpServerBuilder`) for fluent HTTP + observability configuration.
//! - Manual registry using final service labels (no duplicate static label provider).
//! - Separate registry for worker metrics (component="worker") while HTTP + health use component="http".
//! - Service lifecycle (`initialize`, `start`, `shutdown`) registering and exercising custom metrics.
//! - Background task observing a histogram and manipulating a gauge (slowed rate).
//! - Graceful shutdown aborts background task and zeros gauges.
//!
//! Metrics exposed (all prefixed with common labels):
//! - `integration_health_checks_total` (counter, component="http")
//! - `integration_worker_tasks_active` (gauge, component="worker")
//! - `integration_worker_task_duration_seconds_bucket|sum|count` (histogram family, component="worker")
//! - Standard HTTP / readiness metrics (e.g. `http_ready`)
//!
//! Run (interactive):
//! ```powershell
//! # let the OS pick an ephemeral port (default)
//! cargo run --manifest-path crates/examples/Cargo.toml --bin custom_metrics_registry
//! # or for deterministic testing, set an explicit port via env var
//! $env:CUSTOM_METRICS_PORT = "9001"
//! cargo run --manifest-path crates/examples/Cargo.toml --bin custom_metrics_registry
//! ```
//! When `CUSTOM_METRICS_PORT` is set the binary binds the requested port; otherwise it
//! falls back to `0` (ephemeral port) and prints the bound address to stdout.
//! Then query metrics:
//! ```bash
//! curl http://127.0.0.1:<PORT>/metrics | grep integration_
//! ```
//! Trigger health increments:
//! ```bash
//! curl http://127.0.0.1:<PORT>/healthz
//! ```
//! Shut down with Ctrl+C to exercise graceful cleanup.

use std::sync::Arc;
use tokio::sync::Mutex;

use commons::services::robotorq_service::{HttpServerBuilder, RoboTorqService};
use commons::util::error::{ExampleError, ServiceError};
use commons::util::logging::spawn_traced;

/// Integration service demonstrating lifecycle + custom metrics.
struct IntegrationService {
    // Worker registry separate from HTTP/common labels; HTTP metrics rely on server registry, so no local field needed.
    registry_worker: Arc<dyn commons::util::metrics::MetricsRegistry>,
    health_counter: Arc<dyn commons::util::metrics::MetricCounter + Send + Sync>,
    active_tasks_gauge: Arc<dyn commons::util::metrics::MetricGauge + Send + Sync>,
    task_duration_hist: Arc<dyn commons::util::metrics::MetricHistogram + Send + Sync>,
    tasks: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl IntegrationService {
    fn new(http_registry: Arc<dyn commons::util::metrics::MetricsRegistry>, version: &str) -> Self {
        // Worker registry uses component="worker" distinct from HTTP component.
        let worker_registry: Arc<dyn commons::util::metrics::MetricsRegistry> =
            Arc::new(commons::util::metrics::PrometheusRegistry::new(
                "integration_service",
                "worker",
                version,
            ));
        let health_counter = http_registry
            .counter(
                "integration_health_checks_total",
                "Total health checks",
                &[],
            )
            .into();
        let active_tasks_gauge = worker_registry
            .gauge(
                "integration_worker_tasks_active",
                "Active worker tasks",
                &[],
            )
            .into();
        let task_duration_hist = worker_registry
            .histogram(
                "integration_worker_task_duration_seconds",
                "Worker task synthetic duration",
                &[],
                Some(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5]),
            )
            .into();
        Self {
            registry_worker: worker_registry,
            health_counter,
            active_tasks_gauge,
            task_duration_hist,
            tasks: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

// Explicitly implement the small contributor traits required by `RoboTorqService`.
impl commons::services::robotorq_service::ServiceLifecycle for IntegrationService {}

impl commons::services::robotorq_service::HealthContributor for IntegrationService {
    fn health_status(&self) -> String {
        self.health_check()
            .unwrap_or_else(|e| format!("error: {:?}", e))
    }
}

impl commons::services::robotorq_service::MetricsContributor for IntegrationService {
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
        <IntegrationService as commons::services::robotorq_service::RoboTorqService>::export_metrics(
            self,
        )
    }
}

impl RoboTorqService for IntegrationService {
    fn health_check(&self) -> Result<String, ServiceError> {
        self.health_counter.inc();
        Ok("ok".to_string())
    }
    // Export worker metrics (second registry) text so both sets appear.
    fn export_metrics(&self) -> String {
        self.registry_worker.export_text()
    }
    async fn initialize(
        &mut self,
        _cfg: &commons::util::config::RoboTorqConfig,
    ) -> Result<(), ServiceError> {
        self.active_tasks_gauge.set(0.0);
        Ok(())
    }
    async fn start(&self) -> Result<(), ServiceError> {
        let gauge = Arc::clone(&self.active_tasks_gauge);
        let hist = Arc::clone(&self.task_duration_hist);
        let tasks_ref = Arc::clone(&self.tasks);
        let handle = spawn_traced("integration_worker", async move {
            // Lightweight, local RNG for the example to avoid dependency/version mismatches
            struct SimpleRng(u64);
            impl SimpleRng {
                fn next(&mut self) -> u64 {
                    // xorshift-ish deterministic step (sufficient for example purposes)
                    self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
                    self.0
                }
                fn gen_range(&mut self, range: std::ops::Range<u64>) -> u64 {
                    let n = self.next();
                    range.start + (n % (range.end - range.start))
                }
            }
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let mut rng = SimpleRng(seed);
            loop {
                gauge.inc();
                let sleep_ms: u64 = rng.gen_range(100..500); // slowed loop rate
                let start = std::time::Instant::now();
                tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                hist.observe(start.elapsed().as_secs_f64());
                gauge.dec();
                // Small idle pause before next task burst
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
        tasks_ref.lock().await.push(handle);
        Ok(())
    }
    async fn shutdown(&self) -> Result<(), ServiceError> {
        let mut tasks = self.tasks.lock().await;
        for t in tasks.drain(..) {
            t.abort();
        }
        self.active_tasks_gauge.set(0.0);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Allow an explicit port for deterministic testing via `CUSTOM_METRICS_PORT`.
    // If unset or invalid, fall back to 0 (ephemeral port).
    let port: u16 = std::env::var("CUSTOM_METRICS_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Construct HTTP registry with final labels (integration service, component http).
    let http_registry: Arc<dyn commons::util::metrics::MetricsRegistry> = Arc::new(
        commons::util::metrics::PrometheusRegistry::new("integration_service", "http", "0.0.1-int"),
    );
    let service = Arc::new(Mutex::new(IntegrationService::new(
        Arc::clone(&http_registry),
        "0.0.1-int",
    )));
    HttpServerBuilder::new(service)
        .with_port(port)
        .with_registry(Arc::clone(&http_registry))
        // Static labels unnecessary; using manual registry labels directly.
        .build_and_start_autoload()
        .await
        .map_err(|e| ExampleError::Other(format!("service failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_metrics_present() {
        let http_reg: Arc<dyn commons::util::metrics::MetricsRegistry> =
            Arc::new(commons::util::metrics::PrometheusRegistry::new(
                "integration_service",
                "http",
                "0.0.1-int",
            ));
        let svc = IntegrationService::new(Arc::clone(&http_reg), "0.0.1-int");
        // Simulate a health check.
        let _ = svc.health_check();
        // Touch worker metrics so they are guaranteed to be emitted.
        svc.active_tasks_gauge.set(0.0);
        svc.task_duration_hist.observe(0.01);
        let worker_text = svc.export_metrics();
        let http_text = http_reg.export_text();
        assert!(http_text.contains("integration_health_checks_total"));
        assert!(worker_text.contains("integration_worker_tasks_active"));
        assert!(worker_text.contains("integration_worker_task_duration_seconds_bucket"));
        // Distinct component labels present.
        assert!(worker_text.contains("component=\"worker\""));
        assert!(http_text.contains("component=\"http\""));
    }

    #[tokio::test]
    async fn start_and_shutdown_server_exercises_lifecycle() {
        // Build HTTP registry and integration service
        let http_reg: Arc<dyn commons::util::metrics::MetricsRegistry> =
            Arc::new(commons::util::metrics::PrometheusRegistry::new(
                "integration_service",
                "http",
                "0.0.1-test",
            ));

        let svc = Arc::new(Mutex::new(IntegrationService::new(
            Arc::clone(&http_reg),
            "0.0.1-test",
        )));

        // Reserve an available ephemeral port by binding and releasing a std TcpListener.
        let std_listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral");
        let port = std_listener.local_addr().unwrap().port();
        drop(std_listener);

        // Build the server on the reserved port and attach the HTTP registry so
        // middleware + /metrics use the same registry.
        let builder = HttpServerBuilder::new(Arc::clone(&svc))
            .with_port(port)
            .with_registry(Arc::clone(&http_reg));
        let server = builder.build();

        // Use a oneshot to drive shutdown from the test thread after server binds.
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn the server; it will run until we send the oneshot signal.
        let handle = spawn_traced("test_server", async move {
            let shutdown_fut = async move {
                let _ = rx.await;
            };
            server.start_with_shutdown(shutdown_fut).await
        });

        // Allow some time for the server to bind and run initialization/start hooks.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Perform simple HTTP checks against /healthz and /metrics to exercise handlers.
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/healthz", port);
        let metrics_url = format!("http://127.0.0.1:{}/metrics", port);
        let hres = client
            .get(&health_url)
            .send()
            .await
            .expect("health request failed");
        assert!(hres.status().is_success());
        let mres = client
            .get(&metrics_url)
            .send()
            .await
            .expect("metrics request failed");
        assert!(mres.status().is_success());

        // Signal shutdown and wait for server to exit gracefully.
        let _ = tx.send(());
        let result = handle.await.expect("server task panicked");
        assert!(
            result.is_ok(),
            "server did not shut down cleanly: {:?}",
            result
        );
    }
}
