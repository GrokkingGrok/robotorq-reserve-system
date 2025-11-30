//! Example RoboTorq service demonstrating Phase 1 HTTP server and configuration.
//!
//! This example shows how to implement a RoboTorqService that exposes HTTP endpoints
//! with health checks, metrics, and proper configuration loading. It demonstrates
//! the complete Phase 1 observability stack and configuration schema.

use commons::services::http::{HttpServer, HttpServerConfig, RoboTorqService};
use commons::util::config::{load_robotorq_config, RoboTorqConfig, CryptoConfig, EconomicConfig};
use commons::util::metrics::MetricsHandler;
use commons::util::error::{InvariantError, config_error::ConfigError};
use std::sync::Arc;

/// Example RoboTorq service implementation.
///
/// This service demonstrates the standard lifecycle and HTTP interface
/// that all RoboTorq services should implement.
#[derive(Clone)]
pub struct ExampleService {
    /// Metrics handler for collecting and exposing Prometheus metrics.
    metrics: Arc<MetricsHandler>,
    /// Service configuration.
    config: RoboTorqConfig,
    /// Service health status.
    healthy: Arc<std::sync::atomic::AtomicBool>,
}

impl ExampleService {
    /// Create a new example service with the given configuration.
    ///
    /// Initializes the service with metrics collection and health tracking.
    /// Registers example metrics for demonstration purposes.
    ///
    /// # Arguments
    ///
    /// * `config` - Complete RoboTorq configuration including HTTP, NATS, persistence,
    ///   observability, and security settings. Must have valid schema version.
    ///
    /// # Returns
    ///
    /// Returns a new `ExampleService` instance ready for initialization.
    ///
    /// # Panics
    ///
    /// This function does not panic under normal circumstances.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::{load_robotorq_config, RoboTorqConfig};
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Load configuration from default location
    /// let config = load_robotorq_config(None)?;
    ///
    /// // Create service instance
    /// let service = ExampleService::new(config);
    ///
    /// // Service is now ready for initialization
    /// assert!(!service.healthy.load(std::sync::atomic::Ordering::Relaxed));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: RoboTorqConfig) -> Self {
        let metrics = MetricsHandler::new();

        // Register some example metrics
        metrics.register_counter("example_requests_total", "Total number of example requests");
        metrics.register_gauge("example_active_connections", "Number of active connections");

        Self {
            metrics,
            config,
            healthy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl RoboTorqService for ExampleService {
    /// Health check implementation.
    ///
    /// Returns the service health status. In a real service, this would check
    /// database connections, external dependencies, etc.
    ///
    /// # Arguments
    ///
    /// This method takes no arguments - it operates on the service instance.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` with a health status message if the service is healthy,
    /// or `Err(InvariantError)` if the service is not ready or has issues.
    ///
    /// # Panics
    ///
    /// This method does not panic - all errors are returned as `InvariantError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::RoboTorqConfig;
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RoboTorqConfig::default();
    /// let mut service = ExampleService::new(config.clone());
    ///
    /// // Before initialization - should fail
    /// assert!(service.health_check().is_err());
    ///
    /// // After initialization - should succeed
    /// service.initialize(&config).await?;
    /// assert!(service.health_check().is_ok());
    /// let status = service.health_check().unwrap();
    /// assert_eq!(status, "Example service is healthy");
    /// # Ok(())
    /// # }
    /// ```
    fn health_check(&self) -> Result<String, InvariantError> {
        if self.healthy.load(std::sync::atomic::Ordering::Relaxed) {
            Ok("Example service is healthy".to_string())
        } else {
            Err(InvariantError::Config(
                ConfigError::Invalid("Service not yet initialized".to_string())
            ))
        }
    }

    /// Metrics export implementation.
    ///
    /// Returns Prometheus-formatted metrics for monitoring and alerting.
    ///
    /// # Arguments
    ///
    /// This method takes no arguments - it operates on the service instance.
    ///
    /// # Returns
    ///
    /// Returns a `String` containing Prometheus-formatted metrics data.
    /// Includes all registered metrics with current values and metadata.
    ///
    /// # Panics
    ///
    /// This method does not panic under normal circumstances.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::RoboTorqConfig;
    /// use example_service::ExampleService;
    ///
    /// let config = RoboTorqConfig::default();
    /// let service = ExampleService::new(config);
    ///
    /// // Export metrics in Prometheus format
    /// let metrics = service.export_metrics();
    /// assert!(metrics.contains("# HELP"));
    /// assert!(metrics.contains("# TYPE"));
    /// ```
    fn export_metrics(&self) -> String {
        self.metrics.export_text()
    }

    /// Initialize the service.
    ///
    /// This is called during service startup to set up resources and dependencies.
    /// In Phase 2, this would connect to databases, NATS, etc.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the complete RoboTorq configuration. Used to validate
    ///   settings and configure connections. Must match the config passed to `new()`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization succeeds, or `Err(InvariantError)` if
    /// configuration is invalid or resources cannot be initialized.
    ///
    /// # Panics
    ///
    /// This method does not panic - all errors are returned as `InvariantError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::{RoboTorqConfig, Mode};
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RoboTorqConfig {
    ///     schema_version: 1,
    ///     mode: Mode::Production,
    ///     ..RoboTorqConfig::default()
    /// };
    ///
    /// let mut service = ExampleService::new(config.clone());
    ///
    /// // Initialize service with validated config
    /// service.initialize(&config).await?;
    ///
    /// // Service is now healthy and ready
    /// assert!(service.health_check().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    async fn initialize(&mut self, config: &RoboTorqConfig) -> Result<(), InvariantError> {
        tracing::info!("Initializing example service...");

        // Validate configuration
        if config.schema_version == 0 {
            return Err(InvariantError::Config(
                ConfigError::Invalid("Schema version cannot be 0".to_string())
            ));
        }

        // In Phase 2: Connect to database, NATS, etc.
        // For now, just simulate initialization
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.healthy.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Example service initialized successfully");
        Ok(())
    }

    /// Start the service.
    ///
    /// Transition from initialized to running state. Begin processing requests.
    ///
    /// # Arguments
    ///
    /// This method takes no arguments - it operates on the initialized service instance.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the service starts successfully, or `Err(InvariantError)` if
    /// startup fails (e.g., cannot bind to ports, background tasks fail to start).
    ///
    /// # Panics
    ///
    /// This method does not panic - all errors are returned as `InvariantError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::RoboTorqConfig;
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RoboTorqConfig::default();
    /// let mut service = ExampleService::new(config.clone());
    ///
    /// // Must initialize before starting
    /// service.initialize(&config).await?;
    ///
    /// // Start the service to begin processing
    /// service.start().await?;
    ///
    /// // Service is now running and accepting work
    /// # Ok(())
    /// # }
    /// ```
    async fn start(&self) -> Result<(), InvariantError> {
        tracing::info!("Starting example service...");

        // In Phase 2: Start background tasks, NATS subscriptions, etc.
        // For now, just log that we're starting
        tracing::info!("Example service started and ready to serve requests");
        Ok(())
    }

    /// Stop the service gracefully.
    ///
    /// Stop accepting new work but complete in-flight operations.
    ///
    /// # Arguments
    ///
    /// This method takes no arguments - it operates on the running service instance.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the service stops gracefully, or `Err(InvariantError)` if
    /// shutdown encounters errors (e.g., cannot drain queues, background tasks fail).
    ///
    /// # Panics
    ///
    /// This method does not panic - all errors are returned as `InvariantError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::RoboTorqConfig;
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RoboTorqConfig::default();
    /// let mut service = ExampleService::new(config.clone());
    ///
    /// service.initialize(&config).await?;
    /// service.start().await?;
    ///
    /// // Gracefully stop the service
    /// service.stop().await?;
    ///
    /// // Service has stopped accepting new work
    /// # Ok(())
    /// # }
    /// ```
    async fn stop(&self) -> Result<(), InvariantError> {
        tracing::info!("Stopping example service...");

        // In Phase 2: Signal background tasks to stop, drain queues, etc.
        tracing::info!("Example service stopped");
        Ok(())
    }

    /// Shut down the service completely.
    ///
    /// Clean up all resources and prepare for termination.
    ///
    /// # Arguments
    ///
    /// This method takes no arguments - it operates on the service instance.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if shutdown completes successfully, or `Err(InvariantError)` if
    /// cleanup fails (e.g., cannot close connections, resources leak).
    ///
    /// # Panics
    ///
    /// This method does not panic - all errors are returned as `InvariantError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::util::config::RoboTorqConfig;
    /// use example_service::ExampleService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RoboTorqConfig::default();
    /// let mut service = ExampleService::new(config.clone());
    ///
    /// service.initialize(&config).await?;
    /// service.start().await?;
    /// service.stop().await?;
    ///
    /// // Complete shutdown and cleanup
    /// service.shutdown().await?;
    ///
    /// // Service is fully terminated
    /// assert!(!service.healthy.load(std::sync::atomic::Ordering::Relaxed));
    /// # Ok(())
    /// # }
    /// ```
    async fn shutdown(&self) -> Result<(), InvariantError> {
        tracing::info!("Shutting down example service...");

        // In Phase 2: Close database connections, NATS clients, etc.
        self.healthy.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Example service shut down");
        Ok(())
    }
}

/// Example main function demonstrating service lifecycle.
///
/// This shows the complete service startup sequence that all RoboTorq services
/// should follow: config loading, service creation, initialization, and HTTP server start.
///
/// # Arguments
///
/// This function takes no arguments - configuration is loaded from default locations.
///
/// # Returns
///
/// Returns `Ok(())` if the service starts successfully and runs until terminated,
/// or `Err(Box<dyn std::error::Error>)` if startup fails.
///
/// # Panics
///
/// This function may panic if critical initialization fails (logging setup, etc.).
/// In production, consider using proper error handling instead of panicking.
///
/// # Examples
///
/// ```rust,no_run
/// // Run the example service with default configuration
/// // This will start an HTTP server on port 8080 with health and metrics endpoints
/// example_service::main();
/// ```
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    commons::util::logging::init_logging_pretty("info")
        .map_err(|e| format!("Failed to initialize logging: {}", e))?;


    tracing::info!("Starting Example RoboTorq Service");

    // Load configuration
    // This demonstrates the expanded config schema from Phase 1
    let config = load_robotorq_config(None)?;
    tracing::info!("Loaded configuration with schema version {}", config.schema_version);

    // Create service instance
    let mut service = ExampleService::new(config.clone());

    // Initialize service (Phase 2: this will connect to databases, NATS, etc.)
    service.initialize(&config).await?;
    service.start().await?;

    // Configure HTTP server with Phase 1 features
    let http_config = HttpServerConfig::local_defaults(8080);
    let http_server = HttpServer::new(Arc::new(service), http_config);

    // Start HTTP server (provides /healthz, /readyz, /metrics endpoints)
    tracing::info!("Starting HTTP server on http://127.0.0.1:8080");
    tracing::info!("Health check: http://127.0.0.1:8080/healthz");
    tracing::info!("Metrics: http://127.0.0.1:8080/metrics");

    http_server.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use commons::util::config::{Mode, Simulation, HttpConfig, NatsConfig, PersistenceConfig, ObservabilityConfig, SecurityConfig};

    /// Test service creation with valid configuration.
    ///
    /// This test verifies that a service can be created with a complete configuration
    /// and starts in an uninitialized (unhealthy) state.
    ///
    /// # Testing Approach
    ///
    /// - Create a complete RoboTorqConfig with all required sections
    /// - Instantiate ExampleService with this config
    /// - Verify the service starts unhealthy (not yet initialized)
    ///
    /// # Input
    ///
    /// - `config`: Complete RoboTorqConfig with schema_version=1, production mode,
    ///   and default settings for all sections (HTTP, NATS, persistence, observability, security)
    ///
    /// # Expected Output
    ///
    /// - Service is created successfully
    /// - Service health status is `false` (not initialized)
    /// - No panics or errors during construction
    #[test]
    fn example_service_creation() {
        let config = RoboTorqConfig {
            schema_version: 1,
            mode: Mode::Production,
            simulation: Simulation::default(),
            ports: commons::util::config::load_ports_config_from_default(),
            http: HttpConfig::default(),
            nats: NatsConfig::default(),
            persistence: PersistenceConfig::default(),
            observability: ObservabilityConfig::default(),
            security: SecurityConfig::default(),
            crypto: CryptoConfig::default(),
            economic: EconomicConfig::default(),
        };

        let service = ExampleService::new(config);
        assert!(!service.healthy.load(std::sync::atomic::Ordering::Relaxed));
    }

    /// Test complete service lifecycle from creation to shutdown.
    ///
    /// This test verifies the full service lifecycle: creation, initialization,
    /// starting, stopping, and shutdown, ensuring health checks work correctly.
    ///
    /// # Testing Approach
    ///
    /// - Create service with simulation mode config
    /// - Test health check fails before initialization
    /// - Initialize service and verify health check passes
    /// - Start service and verify it runs
    /// - Stop service gracefully
    /// - Shutdown completely and verify health check fails again
    ///
    /// # Input
    ///
    /// - `config`: RoboTorqConfig with schema_version=1, simulation mode,
    ///   and default settings for all configuration sections
    ///
    /// # Expected Output
    ///
    /// - Before initialization: health_check() returns Err
    /// - After initialization: health_check() returns Ok with success message
    /// - After shutdown: health_check() returns Err again
    /// - All lifecycle methods (initialize, start, stop, shutdown) return Ok(())
    /// - No panics during the complete lifecycle
    #[tokio::test]
    async fn example_service_lifecycle() {
        let config = RoboTorqConfig {
            schema_version: 1,
            mode: Mode::Simulation,
            simulation: Simulation::default(),
            ports: commons::util::config::load_ports_config_from_default(),
            http: HttpConfig::default(),
            nats: NatsConfig::default(),
            persistence: PersistenceConfig::default(),
            observability: ObservabilityConfig::default(),
            security: SecurityConfig::default(),
            crypto: CryptoConfig::default(),
            economic: EconomicConfig::default(),
        };

        let mut service = ExampleService::new(config.clone());

        // Should fail health check before initialization
        assert!(service.health_check().is_err());

        // Initialize
        service.initialize(&config).await.unwrap();
        assert!(service.healthy.load(std::sync::atomic::Ordering::Relaxed));

        // Should pass health check after initialization
        assert!(service.health_check().is_ok());

        // Start service
        service.start().await.unwrap();

        // Stop service
        service.stop().await.unwrap();

        // Shutdown
        service.shutdown().await.unwrap();
        assert!(!service.healthy.load(std::sync::atomic::Ordering::Relaxed));
    }
}