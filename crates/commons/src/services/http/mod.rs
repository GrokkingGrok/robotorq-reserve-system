//! Axum HTTP utilities for exposing `RoboTorqService` implementations.
//!
//! Handlers and lifecycle helpers live in focused submodules: `healthz`,
//! `readyz`, `metrics`, `initialization`, and `shutdown`. Prefer those over
//! adding logic here.
#![allow(async_fn_in_trait)]
use crate::util::config::RoboTorqConfig;
use crate::util::error::{InvariantError, logging_error::LoggingError};
use axum::{Router, routing::get, Extension};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
// Metrics abstraction for HTTP middleware wiring
// Metrics abstraction imported when wiring middleware
// use crate::util::metrics::{MetricsRegistry, Histogram, Counter, Gauge};
// use std::time::Duration;

pub mod healthz;
mod initialization;
mod metrics;
pub mod middleware;
pub mod readyz;
mod shutdown;
pub use healthz::health_handler;
pub use initialization::{initialize_service, load_and_initialize_service};
pub use metrics::metrics_handler;
pub use shutdown::{ctrl_c_signal, ctrl_c_signal_with_service_shutdown};

/// Core trait for services exposed via standardized HTTP endpoints.
pub trait RoboTorqService: Send + Sync + 'static {
    /// Liveness check; `GET /healthz` returns 200 when this is Ok.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if the service is not healthy or encounters
    /// an error during the health check. The specific error depends on the
    /// service implementation.
    fn health_check(&self) -> Result<String, InvariantError>;

    /// Export metrics in Prometheus text format; served at `/metrics`.
    fn export_metrics(&self) -> String;

    /// Optional hook for custom endpoints beyond `/healthz` and `/metrics`.
    fn handle_request(&self, _path: &str, _method: &str) -> Option<Result<String, InvariantError>> {
        None
    }

    /// One-time initialization with configuration and dependencies.
    ///
    /// This method is called once during service startup to set up resources,
    /// establish connections, load configuration, and prepare for operation.
    /// The service should validate its configuration and set up any required
    /// dependencies before returning.
    ///
    /// # Arguments
    ///
    /// * `config` - The system-wide RoboTorq configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization succeeds, or `Err(error)` if
    /// initialization fails (invalid config, connection failures, etc.).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    /// use commons::util::config::RoboTorqConfig;
    ///
    /// let mut gateway = RobotGateway::single(RobotId::new());
    ///
    /// // Before: Service exists but resources not allocated
    /// // Configuration not loaded, connections not established
    ///
    /// let config = RoboTorqConfig::default();
    /// gateway.initialize(&config).await?;
    ///
    /// // After: Service is initialized
    /// // - Configuration validated and applied
    /// // - Database connections established
    /// // - Metrics initialized
    /// // - Ready to start processing
    /// # Ok::<(), commons::util::error::InvariantError>(())
    /// ```
    fn initialize(&mut self, _config: &RoboTorqConfig) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
        async {
            Ok(())
        }
    }

    /// Transition from initialized to running.
    ///
    /// This method transitions the service from initialized state to running state.
    /// The service should begin accepting requests, processing operations, and
    /// maintaining its operational state. This is separate from initialization
    /// to allow for coordinated startup across multiple services.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the service starts successfully, or `Err(error)` if
    /// startup fails (resource allocation issues, binding failures, etc.).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    /// use commons::util::config::RoboTorqConfig;
    ///
    /// let mut gateway = RobotGateway::single(RobotId::new());
    /// let config = RoboTorqConfig::default();
    ///
    /// // Initialize first
    /// gateway.initialize(&config).await?;
    ///
    /// // Before: Service is initialized but not processing requests
    /// // No background tasks running, no request handling
    ///
    /// // Start the service
    /// gateway.start().await?;
    ///
    /// // After: Service is running
    /// // - Background processing tasks started
    /// // - Request handling active
    /// // - Metrics collection running
    /// // - Ready to serve clients
    /// # Ok::<(), commons::util::error::InvariantError>(())
    /// ```
    fn start(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
        async {
            Ok(())
        }
    }

    /// Gracefully stop processing while keeping resources allocated.
    ///
    /// This method gracefully stops the service's operational processing but
    /// keeps resources allocated for potential restart. The service should
    /// stop accepting new requests, complete in-flight operations, and enter
    /// a paused state. This allows for quick restart without full re-initialization.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the service stops successfully, or `Err(error)` if
    /// the stop operation fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    /// use commons::util::config::RoboTorqConfig;
    ///
    /// let mut gateway = RobotGateway::single(RobotId::new());
    /// let config = RoboTorqConfig::default();
    ///
    /// gateway.initialize(&config).await?;
    /// gateway.start().await?;
    ///
    /// // Service is running and processing requests
    /// assert!(gateway.health_check().is_ok());
    ///
    /// // Stop processing
    /// gateway.stop().await?;
    ///
    /// // After: Service is stopped
    /// // - No longer accepting new requests
    /// - Completed in-flight operations
    /// // - Resources still allocated
    /// // - Can be restarted quickly
    /// # Ok::<(), commons::util::error::InvariantError>(())
    /// ```
    fn stop(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
        async {
            Ok(())
        }
    }

    /// Final cleanup; release all resources.
    ///
    /// This method performs a clean shutdown of the service, releasing all
    /// allocated resources, closing connections, and preparing for termination.
    /// After shutdown, the service cannot be restarted and should be discarded.
    /// This is the final cleanup method in the service lifecycle.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if shutdown completes successfully, or `Err(error)` if
    /// shutdown encounters issues (resource cleanup failures, etc.).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    /// use commons::util::config::RoboTorqConfig;
    ///
    /// let mut gateway = RobotGateway::single(RobotId::new());
    /// let config = RoboTorqConfig::default();
    ///
    /// gateway.initialize(&config).await?;
    /// gateway.start().await?;
    /// gateway.stop().await?;
    ///
    /// // Before: Service is stopped but resources allocated
    /// // Connections open, memory allocated, files open
    ///
    /// // Complete shutdown
    /// gateway.shutdown().await?;
    ///
    /// // After: Service is fully shut down
    /// // - All connections closed
    /// // - Memory deallocated
    /// // - Files closed
    /// // - Service cannot be restarted
    /// # Ok::<(), commons::util::error::InvariantError>(())
    /// ```
    fn shutdown(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
        async {
            Ok(())
        }
    }
}

/// Lightweight Axum server exposing standardized endpoints for a service.
///
/// The HttpServer automatically creates HTTP endpoints for any service that
/// implements `RoboTorqService`. It provides standard `/health` and `/metrics`
/// endpoints, CORS support, and proper error handling.
///
/// # Type Parameters
///
/// * `S` - The service type that implements `RoboTorqService`
///
/// # Fields
///
/// * `service` - The service instance wrapped in an Arc<Mutex> for thread-safe mutable access
/// * `config` - Configuration specifying address, port, and endpoint paths
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use commons::services::http::{HttpServer, HttpServerConfig};
/// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
/// use commons::types::ids::RobotId;
/// use commons::util::config::RoboTorqConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Before: Service exists but is not exposed via HTTP
///     let metrics = RobotGatewayMetrics::new("gateway");
///     let mut gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);
///     let config = RoboTorqConfig::default();
///
///     // After: HTTP server is created and ready to expose the service
///     let server = HttpServer::new(Arc::new(Mutex::new(gateway)), HttpServerConfig::local_defaults(8080));
///     server.start(&config).await?;
///
///     Ok(())
/// }
/// ```
pub struct HttpServer<S: RoboTorqService> {
    /// The service instance wrapped in an Arc<Mutex> for thread-safe mutable access across HTTP requests.
    service: Arc<Mutex<S>>,
    /// Configuration specifying network address, port, and endpoint paths.
    config: HttpServerConfig,
    /// Readiness flag indicating whether the server is ready to serve traffic.
    ready: Arc<AtomicBool>,
    // Optional metrics registry for middleware; can be None in minimal setups
    // (Will be extended in Phase 1 wiring.)
}

impl<S: RoboTorqService> HttpServer<S> {
    /// Create a new server for the given service and config.
    ///
    /// This constructor wraps the service in an Arc<Mutex> for thread-safe mutable access
    /// across multiple HTTP requests and stores the configuration.
    ///
    /// # Arguments
    ///
    /// * `service` - The service instance to expose via HTTP, wrapped in an Arc<Mutex>
    /// * `config` - HTTP server configuration specifying address, port, and endpoints
    ///
    /// # Returns
    ///
    /// A new HttpServer instance ready to be started.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    /// use commons::services::http::{HttpServer, HttpServerConfig};
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    ///
    /// // Before: Service exists in memory
    /// let metrics = RobotGatewayMetrics::new("gateway");
    /// let gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);
    ///
    /// // After: HTTP server is created and ready to expose the service
    /// let server = HttpServer::new(Arc::new(Mutex::new(gateway)), HttpServerConfig::local_defaults(8080));
    /// // The service is now wrapped and configured for HTTP exposure
    /// ```
    pub fn new(service: Arc<Mutex<S>>, config: HttpServerConfig) -> Self {
        Self {
            service,
            config,
            ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Bind, route, and serve until shutdown.
    ///
    /// This method orchestrates the full service lifecycle: initialize, start, HTTP serving,
    /// and graceful shutdown (stop + shutdown). It binds to the configured address and port,
    /// sets up the HTTP routes, and begins accepting connections. The server will run until
    /// it receives a shutdown signal or encounters an unrecoverable error.
    ///
    /// The server automatically creates the following endpoints:
    /// - `GET /healthz` - Liveness: returns 200 if process is up
    /// - `GET /readyz` - Readiness: returns 200 only when ready flag is set
    /// - `GET {metrics_path}` - Calls `service.export_metrics()` and returns Prometheus format
    ///
    /// # Arguments
    ///
    /// * `config` - The system-wide RoboTorq configuration for service initialization
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the server shuts down gracefully, or `Err(error)` if
    /// it fails to start or encounters an unrecoverable error.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if the server fails to bind to the configured
    /// address and port, or if the HTTP server encounters an unrecoverable error.
    ///
    /// # Panics
    ///
    /// This method does not panic under normal circumstances. Network binding errors
    /// and HTTP server errors are returned as `InvariantError` results.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    /// use commons::services::http::{HttpServer, HttpServerConfig};
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    /// use commons::util::config::RoboTorqConfig;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let metrics = RobotGatewayMetrics::new("gateway");
    ///     let gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);
    ///     let http_config = HttpServerConfig::local_defaults(8080);
    ///     let robo_config = RoboTorqConfig::default();
    ///     let server = HttpServer::new(Arc::new(Mutex::new(gateway)), http_config);
    ///
    ///     // Before: Server is configured but not running
    ///     // Network port 8080 is available
    ///
    ///     // After: Server is running and accepting connections
    ///     // GET http://127.0.0.1:8080/health returns health status
    ///     // GET http://127.0.0.1:8080/metrics returns Prometheus metrics
    ///     server.start(&robo_config).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn start(self, config: &RoboTorqConfig) -> Result<(), InvariantError> {
        // Initialize the service
        {
            let mut service = self.service.lock().await;
            if let Err(err) = service.initialize(config).await {
                tracing::warn!(error = ?err, "service initialize failed, continuing");
            }
        }

        // Start the service
        {
            let service = self.service.lock().await;
            service.start().await?;
        }

        // Create shutdown signal that will call stop and shutdown
        let shutdown_signal = ctrl_c_signal_with_service_shutdown(Arc::clone(&self.service));

        // Start HTTP server with shutdown
        self.start_with_shutdown(shutdown_signal).await
    }

    /// Bind, route, and serve using config auto-loaded inside commons.
    ///
    /// This convenience method keeps callers clean by loading `RoboTorqConfig`
    /// within commons and driving the full lifecycle. Errors during
    /// initialization are logged and the server continues, per policy.
    pub async fn start_autoload(self) -> Result<(), InvariantError> {
        // Initialize with autoloaded config
        {
            let mut service = self.service.lock().await;
            match load_and_initialize_service(&mut *service).await {
                Ok(_cfg) => {}
                Err(err) => {
                    tracing::warn!(error = ?err, "service initialize failed, continuing");
                }
            }
        }

        // Start the service
        {
            let service = self.service.lock().await;
            service.start().await?;
        }

        // Create shutdown signal that will call stop and shutdown
        let shutdown_signal = ctrl_c_signal_with_service_shutdown(Arc::clone(&self.service));

        // Start HTTP server with shutdown
        self.start_with_shutdown(shutdown_signal).await
    }

    /// Start the HTTP server and shut it down when the provided future resolves.
    ///
    /// This mirrors `start` but allows callers to drive graceful shutdown from
    /// an existing signal source (Ctrl+C, health failures, etc.) instead of
    /// relying on a single internal listener.
    pub async fn start_with_shutdown<F>(self, shutdown_signal: F) -> Result<(), InvariantError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr = format!(
            "{}:{}",
            self.config.service.address, self.config.service.port
        );

        // Build the application with routes
        let ready_flag = Arc::clone(&self.ready);
        let mut app = Router::new()
            .route("/healthz", get(health_handler))
            .route(
                "/readyz",
                get(move || readyz::readyz_handler(Arc::clone(&ready_flag))),
            )
            .route(
                self.config.metrics.0.as_str(),
                get(metrics_handler::<S>),
            )
            .with_state(Arc::clone(&self.service));

        // Conditionally add middleware layers based on configuration
        if let Some(body_limit) = self.config.max_body_size_bytes {
            app = app.layer(RequestBodyLimitLayer::new(body_limit));
        }

        if let Some(timeout_secs) = self.config.timeout_seconds {
            app = app.layer(TimeoutLayer::with_status_code(
                axum::http::StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(timeout_secs),
            ));
        }

        if self.config.cors_permissive {
            app = app.layer(CorsLayer::permissive());
        }

        // Attach metrics registry via Extension and enable HTTP metrics middleware when configured
        if let Some(registry) = &self.config.metrics_registry {
            app = app.layer(Extension(std::sync::Arc::clone(registry)));
            // Use route_layer so MatchedPath is set before middleware runs
            app = app.route_layer(middleware::HttpMetricsLayer::new(std::sync::Arc::clone(registry)));
        }

        // Create listener
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

        tracing::info!("HTTP server listening on {}", addr);

        // Mark ready and start serving; export readiness gauge if registry present
        let mut ready_gauge: Option<Arc<dyn crate::util::metrics::MetricGauge + Send + Sync>> = None;
        if let Some(registry) = &self.config.metrics_registry {
            let g = registry.gauge("http_ready", "HTTP server readiness flag", &[]);
            g.set(1.0);
            ready_gauge = Some(g.into());
        }
        self.ready.store(true, Ordering::Relaxed);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

        // After shutdown, mark not ready
        self.ready.store(false, Ordering::Relaxed);
        if let Some(g) = ready_gauge {
            g.set(0.0);
        }

        Ok(())
    }
}

// Re-exported handlers live in submodules

/// Represents an HTTP service configuration with address and port.
///
/// This struct encapsulates the network configuration needed to bind an HTTP server,
/// including the IP address or hostname and the port number.
///
/// # Fields
///
/// * `address` - The network address to bind the HTTP service to (e.g., "127.0.0.1", "0.0.0.0", "localhost")
/// * `port` - The port number to listen on (e.g., 8080, 9000)
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::HttpService;
///
/// // Before: No network configuration exists
///
/// // After: HTTP service is configured to listen on localhost:8080
/// let service = HttpService::new("127.0.0.1", 8080);
/// assert_eq!(service.address, "127.0.0.1");
/// assert_eq!(service.port, 8080);
///
/// // Can also bind to all interfaces
/// let service_all = HttpService::new("0.0.0.0", 9000);
/// ```
#[derive(Clone, Debug)]
pub struct HttpService {
    /// The network address to bind the HTTP service to.
    ///
    /// This can be:
    /// - `"127.0.0.1"` - Listen only on localhost (secure for development)
    /// - `"0.0.0.0"` - Listen on all network interfaces (for production servers)
    /// - `"localhost"` - Hostname resolution to localhost
    /// - Any valid IP address or resolvable hostname
    pub address: String,
    /// The port number to listen on.
    ///
    /// Common choices:
    /// - `8080` - Standard development port
    /// - `80` - Standard HTTP port (requires root/admin privileges)
    /// - `443` - Standard HTTPS port (requires root/admin privileges)
    /// - `0` - Let the OS assign a random available port
    pub port: u16,
}

impl HttpService {
    /// Creates a new HttpService with the given address and port.
    ///
    /// This constructor accepts any type that can be converted into a String
    /// for the address, allowing convenient creation from string literals,
    /// String instances, or other convertible types.
    ///
    /// # Arguments
    ///
    /// * `address` - The network address to bind to (will be converted to String)
    /// * `port` - The port number to listen on
    ///
    /// # Returns
    ///
    /// A new HttpService instance with the specified configuration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::http::HttpService;
    ///
    /// // Before: No HTTP service configuration exists
    ///
    /// // After: HTTP service is configured and ready to use
    /// let service = HttpService::new("127.0.0.1", 8080);
    ///
    /// // Can use string literals
    /// let service1 = HttpService::new("localhost", 3000);
    ///
    /// // Can use String instances
    /// let addr = String::from("0.0.0.0");
    /// let service2 = HttpService::new(addr, 9000);
    ///
    /// // Can use &str references
    /// let addr_ref = "192.168.1.100";
    /// let service3 = HttpService::new(addr_ref, 8081);
    /// ```
    pub fn new<S: Into<String>>(address: S, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }
}

/// Represents an HTTP endpoint path.
///
/// This struct wraps a URL path string and provides convenient construction methods.
/// Endpoint paths should start with "/" and represent the URL path component.
///
/// # Fields
///
/// * `0` - The URL path as a String (e.g., "/health", "/metrics", "/api/v1/status")
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::HttpEndpoint;
///
/// // Before: No endpoint path is defined
///
/// // After: Endpoint path is created and ready for routing
/// let health_endpoint = HttpEndpoint::new("/health");
/// let metrics_endpoint = HttpEndpoint::new("/metrics");
/// let api_endpoint = HttpEndpoint::new("/api/v1/status");
///
/// // Can be used with string literals or String instances
/// let custom_endpoint = HttpEndpoint::new(String::from("/custom"));
/// ```
#[derive(Clone, Debug)]
pub struct HttpEndpoint(pub String);

impl HttpEndpoint {
    /// Creates a new HttpEndpoint with the given path.
    ///
    /// This constructor accepts any type that can be converted into a String,
    /// allowing convenient creation from string literals, String instances,
    /// or other convertible types.
    ///
    /// # Arguments
    ///
    /// * `path` - The URL path for the endpoint (should start with "/")
    ///
    /// # Returns
    ///
    /// A new HttpEndpoint instance with the specified path.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use commons::services::http::HttpEndpoint;
    ///
    /// // Before: No endpoint configuration exists
    ///
    /// // After: Endpoint is created and can be used for routing
    /// let endpoint = HttpEndpoint::new("/health");
    ///
    /// // Standard REST endpoints
    /// let health = HttpEndpoint::new("/health");
    /// let metrics = HttpEndpoint::new("/metrics");
    /// let api_status = HttpEndpoint::new("/api/v1/status");
    ///
    /// // Dynamic endpoint creation
    /// let service_name = "robot-gateway";
    /// let dynamic_endpoint = HttpEndpoint::new(format!("/services/{}", service_name));
    /// ```
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self(path.into())
    }
}

/// Legacy helpers exist for minimal servers; prefer `HttpServer` for new code.
/// Configuration for an HTTP server including service details and endpoint paths.
///
/// This struct combines all the configuration needed to start an HTTP server:
/// network address/port, health check endpoint path, and metrics endpoint path.
///
/// # Fields
///
/// * `service` - The HTTP service configuration (address and port)
/// * `health` - The endpoint path for health checks (e.g., "/health")
/// * `metrics` - The endpoint path for metrics export (e.g., "/metrics")
///
/// # Examples
///
/// ```rust
/// use commons::services::http::{HttpServerConfig, HttpService, HttpEndpoint};
///
/// // Before: No HTTP server configuration exists
///
/// // After: Complete HTTP server configuration is ready
/// let config = HttpServerConfig::new(
///     HttpService::new("127.0.0.1", 8080),
///     HttpEndpoint::new("/health"),
///     HttpEndpoint::new("/metrics")
/// );
///
/// // Or use the convenience method for local development
/// let dev_config = HttpServerConfig::local_defaults(3000);
/// ```
#[derive(Clone)]
pub struct HttpServerConfig {
    /// The HTTP service configuration specifying network address and port.
    pub service: HttpService,
    /// The endpoint path for health checks.
    ///
    /// This path will be routed to call `service.health_check()` and return
    /// the result as an HTTP response.
    pub health: HttpEndpoint,
    /// The endpoint path for metrics export.
    ///
    /// This path will be routed to call `service.export_metrics()` and return
    /// the metrics in Prometheus text format.
    pub metrics: HttpEndpoint,
    /// Optional request timeout in seconds. If None, no timeout is applied.
    pub timeout_seconds: Option<u64>,
    /// Optional maximum request body size in bytes. If None, no limit is applied.
    pub max_body_size_bytes: Option<usize>,
    /// Whether to enable permissive CORS. Defaults to true for development.
    pub cors_permissive: bool,
    /// Optional shared metrics registry; when set, middleware and handler can use it.
    pub metrics_registry: Option<std::sync::Arc<dyn crate::util::metrics::MetricsRegistry>>,
}

impl std::fmt::Debug for HttpServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpServerConfig")
            .field("service", &self.service)
            .field("health", &self.health)
            .field("metrics", &self.metrics)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_body_size_bytes", &self.max_body_size_bytes)
            .field("cors_permissive", &self.cors_permissive)
            .finish()
    }
}

impl HttpServerConfig {
    /// Creates a new HttpServerConfig with the specified service and endpoints.
    ///
    /// This constructor allows full customization of all HTTP server parameters.
    ///
    /// # Arguments
    ///
    /// * `service` - The HTTP service configuration (address and port)
    /// * `health` - The endpoint path for health checks
    /// * `metrics` - The endpoint path for metrics export
    ///
    /// # Returns
    ///
    /// A new HttpServerConfig instance with the specified configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use commons::services::http::{HttpServerConfig, HttpService, HttpEndpoint};
    ///
    /// // Before: Individual configuration components exist
    /// let service = HttpService::new("0.0.0.0", 80);
    /// let health = HttpEndpoint::new("/health");
    /// let metrics = HttpEndpoint::new("/metrics");
    ///
    /// // After: Complete HTTP server configuration is assembled
    /// let config = HttpServerConfig::new(service, health, metrics);
    ///
    /// // The config can now be used to start an HTTP server
    /// // that listens on 0.0.0.0:80 with /health and /metrics endpoints
    /// ```
    #[must_use]
    pub fn new(service: HttpService, health: HttpEndpoint, metrics: HttpEndpoint) -> Self {
        Self {
            service,
            health,
            metrics,
            timeout_seconds: None,
            max_body_size_bytes: None,
            cors_permissive: true,
            metrics_registry: None,
        }
    }

    /// Convenience for local development defaults.
    ///
    /// Creates an HttpServerConfig with sensible defaults for local development:
    /// - Address: "127.0.0.1" (localhost only)
    /// - Health endpoint: "/health"
    /// - Metrics endpoint: "/metrics"
    /// - Port: As specified by the caller
    ///
    /// # Arguments
    ///
    /// * `port` - The port number to listen on
    ///
    /// # Returns
    ///
    /// An HttpServerConfig instance configured for local development.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use commons::services::http::HttpServerConfig;
    ///
    /// // Before: No configuration exists
    ///
    /// // After: HTTP server is configured for local development on port 8080
    /// let config = HttpServerConfig::local_defaults(8080);
    ///
    /// // This is equivalent to:
    /// // HttpServerConfig::new(
    /// //     HttpService::new("127.0.0.1", 8080),
    /// //     HttpEndpoint::new("/health"),
    /// //     HttpEndpoint::new("/metrics")
    /// // );
    ///
    /// // Perfect for development servers
    /// let dev_config = HttpServerConfig::local_defaults(3000);
    /// ```
    #[must_use]
    pub fn local_defaults(port: u16) -> Self {
        Self {
            service: HttpService::new("127.0.0.1", port),
            health: HttpEndpoint::new("/health"),
            metrics: HttpEndpoint::new("/metrics"),
            timeout_seconds: Some(30), // 30 second timeout for development
            max_body_size_bytes: Some(1024 * 1024), // 1MB body limit
            cors_permissive: true,
            metrics_registry: None,
        }
    }

    /// Enable HTTP observability by providing a metrics registry used by middleware and /metrics.
    pub fn with_metrics_registry(
        mut self,
        registry: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry>,
    ) -> Self {
        self.metrics_registry = Some(registry);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::metrics::MetricsHandler;
    // Use a local test service to avoid cross-crate type duplication
    struct TestService {
        handler: Arc<MetricsHandler>,
    }

    impl RoboTorqService for TestService {
        fn health_check(&self) -> Result<String, InvariantError> {
            Ok("OK".to_string())
        }

        fn export_metrics(&self) -> String {
            self.handler.export_text()
        }

        fn initialize(&mut self, _config: &RoboTorqConfig) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
            async {
                Ok(())
            }
        }
        fn start(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
            async {
                Ok(())
            }
        }
        fn stop(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
            async {
                Ok(())
            }
        }
        fn shutdown(&self) -> impl std::future::Future<Output = Result<(), InvariantError>> + Send {
            async {
                Ok(())
            }
        }
    }

    #[test]
    fn http_server_with_test_service() {
        // Create a minimal service that implements RoboTorqService
        let handler = MetricsHandler::new();
        handler.register_counter("test_requests_total", "Total test requests");
        let svc = Arc::new(Mutex::new(TestService { handler }));

        // Create HTTP server config
        let config = HttpServerConfig::local_defaults(0); // Use port 0 for testing

        // Test that we can create the server (without starting it)
        let _server = HttpServer::new(svc, config);
    }

    #[test]
    fn http_server_config_defaults() {
        let config = HttpServerConfig::local_defaults(8080);
        assert_eq!(config.service.address, "127.0.0.1");
        assert_eq!(config.service.port, 8080);
        assert_eq!(config.health.0, "/health");
        assert_eq!(config.metrics.0, "/metrics");
        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.max_body_size_bytes, Some(1024 * 1024));
        assert!(config.cors_permissive);
    }

    #[tokio::test]
    async fn basic_http_server_async() {
        let handler = MetricsHandler::new();
        // Register some test metrics to ensure export_text() is not empty
        handler.register_counter("test_counter", "test counter");
        let _service = HttpService::new("127.0.0.1", 0); // Use port 0 for testing
        let _health = HttpEndpoint::new("/health");
        let _metrics = HttpEndpoint::new("/metrics");

        // Just verify the handler works
        assert!(!handler.export_text().is_empty());
    }
}
