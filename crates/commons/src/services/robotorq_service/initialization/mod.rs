//! Initialization helpers for `robotorq_service` implementations.
//!
//! Overview
//! - `initialize_service`: Calls a service's `initialize` hook and surfaces `ServiceError`.
//! - `load_and_initialize_service`: Loads `RoboTorqConfig`, wires persistence, then initializes.
//!
//! Use these helpers in your service's startup sequence to centralize common
//! bootstrapping steps (config load, optional persistence driver creation,
//! and service initialization), keeping binaries thin and consistent.
//!
//! Quick Start
//! ```no_run
//! use commons::services::robotorq_service::{initialize_service, load_and_initialize_service, RoboTorqService};
//! use commons::util::error::ServiceError;
//!
//! struct MySvc;
//! impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
//! impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".into() } }
//! impl commons::services::robotorq_service::MetricsContributor for MySvc {}
//! impl commons::services::robotorq_service::RoboTorqService for MySvc {
//!     fn export_metrics(&self) -> String { String::new() }
//!     fn health_check(&self) -> Result<String, ServiceError> { Ok("OK".into()) }
//!     async fn initialize(&mut self, _cfg: &commons::util::config::RoboTorqConfig) -> Result<(), ServiceError> { Ok(()) }
//!     async fn start(&self) -> Result<(), ServiceError> { Ok(()) }
//!     async fn stop(&self) -> Result<(), ServiceError> { Ok(()) }
//!     async fn shutdown(&self) -> Result<(), ServiceError> { Ok(()) }
//! }
//! # async fn demo() -> Result<(), ServiceError> {
//! let mut svc = MySvc;
//! // Load config, wire persistence (best effort), then initialize
//! let _cfg = load_and_initialize_service(&mut svc).await?;
//! // Or, if you already have a config: initialize_service(&mut svc, &cfg).await?;
//! # Ok(()) }
//! ```
use crate::services::robotorq_service::RoboTorqService;
use crate::util::config::RoboTorqConfig;
use crate::util::config::load_robotorq_config;
use crate::util::error::ServiceError;
use crate::util::persistence::make_driver;
use std::sync::Arc;
use tracing::info;

/// Initialize a service with the given configuration.
///
/// Calls `svc.initialize(cfg).await` and returns any error surfaced by the
/// service implementation.
///
/// # Arguments
/// - `svc`: Mutable service implementing `RoboTorqService`.
/// - `cfg`: Immutable `RoboTorqConfig` applied during initialization.
///
/// # Returns
/// - `Ok(())` when the service initializes successfully.
/// - `Err(ServiceError)` when the service reports an initialization failure.
///
/// # Errors
/// - Returns `ServiceError` from the service’s `initialize` implementation.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```rust,no_run
/// use commons::services::robotorq_service::{initialize_service, RoboTorqService};
/// use commons::util::error::ServiceError;
/// struct MySvc;
/// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
/// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".to_string() } }
/// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
/// impl RoboTorqService for MySvc {}
/// #[tokio::main]
/// async fn main() -> Result<(), ServiceError> {
///     let mut svc = MySvc;
///     let cfg = commons::util::config::load_robotorq_config(None).unwrap();
///     initialize_service(&mut svc, &cfg).await?;
///     Ok(())
/// }
/// ```
pub async fn initialize_service<S: RoboTorqService>(
    svc: &mut S,
    cfg: &RoboTorqConfig,
) -> Result<(), ServiceError> {
    // Internally prefer the richer `ServiceError` type; service helpers return
    // `ServiceError` and public boundaries no longer map into a single unified
    // error enum.
    match svc.initialize(cfg).await {
        Ok(()) => Ok(()),
        Err(e) => Err(ServiceError::Other(format!("initialize failed: {e}"))),
    }
}

/// Load `RoboTorqConfig`, wire persistence, and initialize a service.
///
/// Centralizes config loading and best-effort persistence driver creation
/// inside commons to keep service binaries lean. If driver creation fails,
/// the service is initialized without persistence and a warning is logged.
///
/// # Arguments
/// - `svc`: Mutable service implementing `RoboTorqService`.
///
/// # Returns
/// - `Ok(RoboTorqConfig)` on success (also indicates initialization succeeded).
/// - `Err(ServiceError)` if config loading or initialization fails.
///
/// # Errors
/// - Returns `ServiceError` if config load fails or the service’s `initialize` fails.
///
/// # Panics
/// - Not expected to panic.
///
/// # Examples
/// ```no_run
/// use commons::services::robotorq_service::{load_and_initialize_service, RoboTorqService};
/// use commons::util::error::ServiceError;
///
/// struct MySvc; 
/// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
/// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".into() } }
/// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
/// impl RoboTorqService for MySvc {
///     fn export_metrics(&self) -> String { String::new() }
///     fn health_check(&self) -> Result<String, ServiceError> { Ok("OK".into()) }
///     async fn initialize(&mut self, _cfg: &commons::util::config::RoboTorqConfig) -> Result<(), ServiceError> { Ok(()) }
///     async fn start(&self) -> Result<(), ServiceError> { Ok(()) }
///     async fn stop(&self) -> Result<(), ServiceError> { Ok(()) }
///     async fn shutdown(&self) -> Result<(), ServiceError> { Ok(()) }
/// }
/// # async fn demo() -> Result<(), ServiceError> {
/// let mut svc = MySvc;
/// let cfg = load_and_initialize_service(&mut svc).await?;
/// assert!(cfg.http.port > 0);
/// # Ok(()) }
/// ```
pub async fn load_and_initialize_service<S: RoboTorqService>(
    svc: &mut S,
) -> Result<RoboTorqConfig, ServiceError> {
    let cfg = load_robotorq_config(None)
        .map_err(|e| ServiceError::Other(format!("config load failed: {e}")))?;
    info!(
        schema_version = cfg.schema_version,
        mode = ?cfg.mode,
        http_address = %cfg.http.address,
        http_port = cfg.http.port,
        "loaded RoboTorq configuration (commons init)"
    );
    // Attempt to construct a configured persistence driver and inject it into
    // the service before running its initialize hook. Failure to create a
    // driver is non-fatal here (service may operate without persistence).
    match make_driver(&cfg.persistence).await {
        Ok(boxed) => {
            let arc: Arc<dyn crate::util::persistence::PersistenceDriver> = Arc::from(boxed);
            svc.set_persistence_driver(Some(arc));
        }
        Err(err) => {
            tracing::warn!(error = ?err, "failed to create persistence driver; continuing without persistence");
            svc.set_persistence_driver(None);
        }
    }

    // Call the internal initializer which returns `ServiceError`.
    initialize_service(svc, &cfg).await?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::config::RoboTorqConfig;

    // Mock service for testing
    struct MockService {
        should_fail: bool,
    }
    impl RoboTorqService for MockService {
        fn export_metrics(&self) -> String {
            String::new()
        }

        fn health_check(&self) -> Result<String, ServiceError> {
            Ok("OK".to_string())
        }

        async fn initialize(&mut self, _cfg: &RoboTorqConfig) -> Result<(), ServiceError> {
            if self.should_fail {
                Err(ServiceError::Other("mock failure".into()))
            } else {
                Ok(())
            }
        }

        async fn start(&self) -> Result<(), ServiceError> {
            Ok(())
        }

        async fn stop(&self) -> Result<(), ServiceError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), ServiceError> {
            Ok(())
        }
    }

    impl crate::services::robotorq_service::ServiceLifecycle for MockService {}
    impl crate::services::robotorq_service::HealthContributor for MockService {
        fn health_status(&self) -> String {
            self.health_check().unwrap_or_default()
        }
    }
    impl crate::services::robotorq_service::MetricsContributor for MockService {
        fn set_metrics_context(
            &mut self,
            _ctx: Option<std::sync::Arc<crate::services::robotorq_service::service_metrics_context::ServiceMetricsContext>>,
        ) {
        }
        fn export_metrics(&self) -> String {
            <MockService as crate::services::robotorq_service::RoboTorqService>::export_metrics(
                self,
            )
        }
    }

    /// Test that initialize_service succeeds when the service initializes successfully.
    #[tokio::test]
    async fn test_initialize_service_success() {
        let mut svc = MockService { should_fail: false };
        let cfg = crate::util::config::load_robotorq_config(None).unwrap();
        let result = initialize_service(&mut svc, &cfg).await;
        assert!(result.is_ok());
    }

    /// Test that initialize_service fails when the service initialization fails.
    #[tokio::test]
    async fn test_initialize_service_failure() {
        let mut svc = MockService { should_fail: true };
        let cfg = crate::util::config::load_robotorq_config(None).unwrap();
        let result = initialize_service(&mut svc, &cfg).await;
        assert!(result.is_err());
    }

    // Note: load_and_initialize_service is harder to test without mocking config loading
    // For now, we test the core initialize_service function
}
