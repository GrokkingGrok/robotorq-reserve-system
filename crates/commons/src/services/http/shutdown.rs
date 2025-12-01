//! Helpers for wiring service lifecycle cleanup to process shutdown events.
//!
//! These helpers make it easy to hook a service `stop`/`shutdown` sequence into
//! a signal such as Ctrl+C so the service can release resources cleanly.
use crate::services::http::RoboTorqService;
use std::sync::Arc;
use tracing::{info, warn};

/// Build a signal listener that resolves once the process receives Ctrl+C.
///
/// This helper wraps `tokio::signal::ctrl_c()` and logs the event so
/// callers can simply pass the returned future into `HttpServer::start_with_shutdown`.
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use commons::services::http::{HttpServer, HttpServerConfig, RoboTorqService, ctrl_c_signal};
///
/// struct MyService;
/// impl RoboTorqService for MyService {
///     fn health_check(&self) -> Result<String, commons::util::error::InvariantError> {
///         Ok("ok".to_string())
///     }
///
///     fn export_metrics(&self) -> String { String::new() }
/// }
///
/// let svc = Arc::new(MyService);
/// let config = HttpServerConfig::local_defaults(8080);
/// HttpServer::new(svc, config)
///     .start_with_shutdown(ctrl_c_signal())
///     .await?;
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
    let svc = service.lock().await;
    if let Err(err) = svc.stop().await {
        warn!(error = ?err, "service stop hook failed");
    }
    let svc = service.lock().await;
    if let Err(err) = svc.shutdown().await {
        warn!(error = ?err, "service shutdown hook failed");
    }
}
