//! Initialization helper wrapping `RoboTorqService::initialize`.
//!
//! Intended for use in service lifecycle orchestration where a concrete
//! `RoboTorqService` must be initialized with a `RoboTorqConfig` before
//! becoming ready.
use crate::services::http::RoboTorqService;
use crate::util::config::RoboTorqConfig;
use crate::util::config::load_robotorq_config;
use crate::util::error::InvariantError;
use crate::util::error::config_error::ConfigError;
use tracing::info;

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

/// Load `RoboTorqConfig` and initialize a service.
///
/// Centralizes config loading inside commons to keep callers clean.
pub async fn load_and_initialize_service<S: RoboTorqService>(
    svc: &mut S,
) -> Result<RoboTorqConfig, InvariantError> {
    let cfg = load_robotorq_config(None).map_err(ConfigError::Invalid)?;
    info!(
        schema_version = cfg.schema_version,
        mode = ?cfg.mode,
        http_address = %cfg.http.address,
        http_port = cfg.http.port,
        "loaded RoboTorq configuration (commons init)"
    );
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

        fn health_check(&self) -> Result<String, InvariantError> {
            Ok("OK".to_string())
        }

        async fn initialize(&mut self, _cfg: &RoboTorqConfig) -> Result<(), InvariantError> {
            if self.should_fail {
                Err(InvariantError::Config(crate::util::error::config_error::ConfigError::Invalid("mock failure".into())))
            } else {
                Ok(())
            }
        }

        async fn start(&self) -> Result<(), InvariantError> {
            Ok(())
        }

        async fn stop(&self) -> Result<(), InvariantError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), InvariantError> {
            Ok(())
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
