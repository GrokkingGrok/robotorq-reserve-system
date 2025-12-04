//! Helpers for wiring service lifecycle cleanup to process shutdown events.
//!
//! These helpers make it easy to hook a service `stop`/`shutdown` sequence into
//! a signal such as Ctrl+C so the service can release resources cleanly.
use super::RoboTorqService;
use crate::util::logging::log_if_waited;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

/// Build a signal listener that resolves once the process receives Ctrl+C.
///
/// This helper wraps `tokio::signal::ctrl_c()` and logs the event so
/// callers can simply pass the returned future into `HttpServer::start_with_shutdown`.
///
/// # Examples
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, RoboTorqService};
/// use commons::services::robotorq_service::shutdown::ctrl_c_signal;
/// use commons::util::config::load_robotorq_config;
/// use commons::util::error::ServiceError;
/// struct MyService;
/// impl commons::services::robotorq_service::ServiceLifecycle for MyService {}
/// impl commons::services::robotorq_service::HealthContributor for MyService { fn health_status(&self) -> String { "OK".to_string() } }
/// impl commons::services::robotorq_service::MetricsContributor for MyService {}
/// impl RoboTorqService for MyService {}
/// #[tokio::main]
/// async fn main() -> Result<(), ServiceError> {
///     let svc = Arc::new(Mutex::new(MyService));
///     let _server = HttpServer::new(svc, HttpServerConfig::local_defaults(0));
///     let _cfg = load_robotorq_config(None).unwrap();
///     let _shutdown_future = ctrl_c_signal();
///     Ok(())
/// }
/// ```
pub async fn ctrl_c_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to register Ctrl+C handler");
    info!("received shutdown signal");
}

/// Build a signal listener that also runs service cleanup hooks after Ctrl+C.
///
/// This helper wraps `ctrl_c_signal` and, once the shutdown trigger fires, calls
/// `stop` followed by `shutdown` on the provided service. Errors during stop or
/// shutdown are logged but do not stop the signal propagation.
pub async fn ctrl_c_signal_with_service_shutdown<S>(service: Arc<tokio::sync::Mutex<S>>)
where
    S: RoboTorqService,
{
    ctrl_c_signal().await;
    run_service_shutdown(service).await;
}

/// Run the `stop` and `shutdown` hooks on the given service.
///
/// Split out for testability so tests can exercise cleanup logic without
/// needing to send an actual Ctrl+C to the process.
#[instrument(skip(service))]
pub async fn run_service_shutdown<S>(service: Arc<tokio::sync::Mutex<S>>)
where
    S: RoboTorqService,
{
    info!("shutdown sequence starting");
    let start = std::time::Instant::now();

    // Acquire lock for stop(); measure wait time for diagnostics.
    let lock_start = std::time::Instant::now();
    let svc = service.lock().await;
    let lock_wait = lock_start.elapsed();
    debug!(lock_wait_ms = %lock_wait.as_millis(), "acquired service mutex for stop");
    // Surface higher-level diagnostic that may use centralized policy
    log_if_waited(
        "service.stop.mutex",
        Instant::now().checked_sub(lock_wait).unwrap(),
        std::time::Duration::from_millis(50),
    );

    info!("calling service.stop()");
    if let Err(err) = svc.stop().await {
        warn!(error = ?err, "service stop hook failed");
    } else {
        info!("service.stop() completed");
    }

    drop(svc);

    // Re-acquire lock for shutdown(); measure wait time.
    let lock_start = std::time::Instant::now();
    let svc = service.lock().await;
    let lock_wait = lock_start.elapsed();
    debug!(lock_wait_ms = %lock_wait.as_millis(), "acquired service mutex for shutdown");
    log_if_waited(
        "service.shutdown.mutex",
        Instant::now().checked_sub(lock_wait).unwrap(),
        std::time::Duration::from_millis(50),
    );

    info!("calling service.shutdown()");
    if let Err(err) = svc.shutdown().await {
        warn!(error = ?err, "service shutdown hook failed");
    } else {
        info!("service.shutdown() completed");
    }

    let dur = start.elapsed();
    info!(duration_ms = %dur.as_millis(), "shutdown sequence completed");

    // If OTLP tracing was enabled, flush exporter buffers so spans are exported
    // before the process exits. This is a no-op when the `otlp` feature is not
    // enabled (guarded by cfg).
    #[cfg(feature = "otlp")]
    {
        tracing::info!("shutting down tracer provider (flushing spans)");
        // Best-effort flush; ignore errors during shutdown.
        opentelemetry::global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::robotorq_service::{
        HealthContributor, MetricsContributor, ServiceLifecycle,
    };
    use std::sync::Once;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DummyService {
        stopped: Arc<AtomicBool>,
        shutdown: Arc<AtomicBool>,
    }

    impl DummyService {
        fn new() -> (Self, Arc<AtomicBool>, Arc<AtomicBool>) {
            let stopped = Arc::new(AtomicBool::new(false));
            let shutdown = Arc::new(AtomicBool::new(false));
            (
                Self {
                    stopped: Arc::clone(&stopped),
                    shutdown: Arc::clone(&shutdown),
                },
                stopped,
                shutdown,
            )
        }
    }

    impl RoboTorqService for DummyService {
        fn health_check(&self) -> Result<String, crate::util::error::ServiceError> {
            Ok("ok".to_string())
        }
        async fn stop(&self) -> Result<(), crate::util::error::ServiceError> {
            self.stopped.store(true, Ordering::SeqCst);
            Ok(())
        }
        async fn shutdown(&self) -> Result<(), crate::util::error::ServiceError> {
            self.shutdown.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    impl ServiceLifecycle for DummyService {}
    impl HealthContributor for DummyService {
        fn health_status(&self) -> String {
            "ok".to_string()
        }
    }
    impl MetricsContributor for DummyService {}

    // Initialize logging for tests once so we can see diagnostic output.
    static TEST_TRACING: Once = Once::new();
    fn init_test_logging() {
        TEST_TRACING.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::from_default_env()
                        .add_directive("info".parse().unwrap()),
                )
                .with_target(false)
                .try_init();
        });
    }

    #[tokio::test]
    async fn run_service_shutdown_calls_hooks() {
        init_test_logging();
        let (dummy, stopped, shutdown) = DummyService::new();
        let svc = Arc::new(tokio::sync::Mutex::new(dummy));
        // Bound the shutdown call to avoid hanging tests; fail fast if it blocks.
        let timeout_dur = std::time::Duration::from_secs(5);
        let res = tokio::time::timeout(timeout_dur, run_service_shutdown(Arc::clone(&svc))).await;
        assert!(res.is_ok(), "run_service_shutdown timed out");
        assert!(stopped.load(Ordering::SeqCst));
        assert!(shutdown.load(Ordering::SeqCst));
    }
}
