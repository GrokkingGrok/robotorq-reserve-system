//! Shutdown helper wrapping `RoboTorqService::shutdown`.
//!
//! Use this to coordinate final cleanup and resource release when terminating a
//! service. This helper delegates to the service's async shutdown routine and
//! surfaces any error produced.
use crate::util::error::InvariantError;
use crate::services::http::RoboTorqService;

/// Request an orderly shutdown via the service's async shutdown routine.
///
/// Calls `RoboTorqService::shutdown().await` and returns any error surfaced by
/// the service. Prefer this cooperative approach over ad-hoc signal handling
/// or hard process termination.
///
/// # Arguments
/// - `svc`: Reference to a type implementing `RoboTorqService`.
///
/// # Returns
/// - `Ok(())` when the service finishes its shutdown procedure.
/// - `Err(InvariantError)` if the service reports a shutdown failure.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,ignore
/// use std::sync::Arc;
/// use commons::services::http::shutdown::shutdown_service;
/// use commons::services::http::RoboTorqService;
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
///     let svc = MySvc;
///     shutdown_service(&svc).await?;
///     Ok(())
/// }
/// ```
pub async fn shutdown_service<S: RoboTorqService>(svc: &S) -> Result<(), InvariantError> {
    svc.shutdown().await
}
