//! Initialization helper wrapping `RoboTorqService::initialize`.
//!
//! Intended for use in service lifecycle orchestration where a concrete
//! `RoboTorqService` must be initialized with a `RoboTorqConfig` before
//! becoming ready.
use crate::services::http::RoboTorqService;
use crate::util::config::RoboTorqConfig;
use crate::util::error::InvariantError;

/// Initialize a service with the given configuration.
///
/// Calls `svc.initialize(cfg).await` and returns any error surfaced by the
/// service implementation.
///
/// # Arguments
/// - `svc`: Mutable reference to the service implementing `RoboTorqService`.
/// - `cfg`: Immutable reference to `RoboTorqConfig` used during init.
///
/// # Returns
/// - `Ok(())` when initialization completes successfully.
/// - `Err(InvariantError)` if the service reports a failure.
///
/// # Errors
///
/// Returns `InvariantError` if the underlying service's `initialize` method fails.
/// The specific error depends on the service implementation.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,ignore
/// use commons::services::http::initialization::initialize_service;
/// use commons::services::http::RoboTorqService;
/// use commons::util::config::RoboTorqConfig;
///
/// # struct MySvc; /* impl RoboTorqService for MySvc { ... } */
/// # impl commons::services::http::RoboTorqService for MySvc {
/// #     fn export_metrics(&self) -> String { String::new() }
/// #     fn health_check(&self) -> Result<String, commons::util::error::InvariantError> { Ok("OK".into()) }
/// #     fn shutdown<'a>(&'a self) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
/// #     fn initialize<'a>(&'a mut self, _cfg: &commons::util::config::RoboTorqConfig) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<(), commons::util::error::InvariantError>> + Send + 'a>> { Box::pin(async { Ok(()) }) }
/// # }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut svc = MySvc;
///     let cfg = RoboTorqConfig::default();
///     initialize_service(&mut svc, &cfg).await?;
///     Ok(())
/// }
/// ```
pub async fn initialize_service<S: RoboTorqService>(
    svc: &mut S,
    cfg: &RoboTorqConfig,
) -> Result<(), InvariantError> {
    svc.initialize(cfg).await
}
