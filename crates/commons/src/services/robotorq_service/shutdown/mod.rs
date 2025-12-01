//! Helpers for wiring service lifecycle cleanup to process shutdown events.
//!
//! These helpers make it easy to hook a service `stop`/`shutdown` sequence into
//! a signal such as Ctrl+C so the service can release resources cleanly.
use super::robotorq_service;
use std::sync::Arc;
use tracing::{info, warn};

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
/// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, robotorq_service};
/// use commons::services::robotorq_service::shutdown::ctrl_c_signal;
/// use commons::util::config::load_robotorq_config;
/// use commons::util::error::InvariantError;
/// struct MyService;
/// impl robotorq_service for MyService {}
/// #[tokio::main]
/// async fn main() -> Result<(), InvariantError> {
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
    S: robotorq_service,
{
    ctrl_c_signal().await;
    let svc = service.lock().await;
    if let Err(err) = svc.stop().await {
        warn!(error = ?err, "service stop hook failed");
    }
    let svc = service.lock().await;
    if let Err(err) = svc.shutdown().await {
        warn!(error = ?err, "service shutdown hook failed");
    }
}
