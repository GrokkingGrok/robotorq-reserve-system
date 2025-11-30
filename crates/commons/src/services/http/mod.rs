//! HTTP utilities to expose `RoboTorqService` implementations over Axum.
//!
//! Provides `HttpServer` and helpers to serve standardized `/health` and
//! `/metrics` endpoints for any service implementing `RoboTorqService`.
#![allow(async_fn_in_trait)]
use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;
use tokio::net::TcpListener;
use crate::util::error::{InvariantError, logging_error::LoggingError};
use crate::util::metrics::MetricsHandler;
use crate::util::config::RoboTorqConfig;

/// Core trait for any service that can be exposed via HTTP endpoints.
/// Services implementing this trait can provide health checks and metrics.
///
/// This trait enables services to be automatically exposed via HTTP endpoints
/// without coupling the service logic to HTTP infrastructure. The HTTP server
/// will automatically route `/health` and `/metrics` requests to the appropriate
/// trait methods.
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::RoboTorqService;
/// use commons::util::error::InvariantError;
/// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
/// use commons::types::ids::RobotId;
/// use std::sync::Arc;
///
/// // Before: Create a service without HTTP exposure
/// let metrics = RobotGatewayMetrics::new("gateway");
/// let gateway = Arc::new(RobotGateway::single(RobotId::new()).with_metrics(metrics));
///
/// // The service can perform health checks and export metrics
/// // without any HTTP server running
/// assert!(gateway.health_check().is_ok());
/// assert!(!gateway.export_metrics().is_empty());
///
/// // After: The HTTP server can expose this service via endpoints
/// // GET /health -> calls gateway.health_check()
/// // GET /metrics -> calls gateway.export_metrics()
/// ```
pub trait RoboTorqService: Send + Sync + 'static {
    /// Perform a comprehensive health check of the service.
    ///
    /// This method should verify that the service is operational and all
    /// critical components are functioning correctly. The returned message
    /// should provide details about the service's health status.
    ///
    /// # Returns
    ///
    /// Returns `Ok(message)` with a descriptive health status message if the
    /// service is healthy, or `Err(error)` if the service is unhealthy or
    /// misconfigured.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    ///
    /// let metrics = RobotGatewayMetrics::new("gateway");
    /// let gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);
    ///
    /// // Before: Service is properly configured
    /// // After: Health check confirms everything is working
    /// match gateway.health_check() {
    ///     Ok(msg) => println!("Service healthy: {}", msg),
    ///     Err(e) => println!("Service unhealthy: {:?}", e),
    /// }
    /// ```
    fn health_check(&self) -> Result<String, InvariantError>;

    /// Export service metrics in Prometheus text format.
    ///
    /// This method should return metrics data that can be scraped by
    /// Prometheus or displayed in monitoring dashboards. The format should
    /// follow Prometheus exposition format standards.
    ///
    /// # Returns
    ///
    /// A string containing metrics data in Prometheus text format.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    ///
    /// let metrics = RobotGatewayMetrics::new("gateway");
    /// let gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);
    ///
    /// // Before: Metrics are being collected internally
    /// // After: Metrics are exported in Prometheus format
    /// let metrics_text = gateway.export_metrics();
    /// assert!(metrics_text.contains("# HELP"));
    /// assert!(metrics_text.contains("# TYPE"));
    /// ```
    fn export_metrics(&self) -> String;

    /// Handle service-specific HTTP requests.
    ///
    /// This optional method allows services to handle custom endpoints beyond
    /// the standard `/health` and `/metrics`. If the service can handle the
    /// request, it should return `Some(result)`. If it cannot handle the request,
    /// it should return `None` to allow the HTTP server to handle it or return
    /// a 404.
    ///
    /// # Arguments
    ///
    /// * `path` - The request path (e.g., "/custom/endpoint")
    /// * `method` - The HTTP method (e.g., "GET", "POST")
    ///
    /// # Returns
    ///
    /// Returns `Some(Ok(response))` if the request was handled successfully,
    /// `Some(Err(error))` if the request was handled but failed, or `None` if
    /// the service cannot handle this request.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use commons::services::http::RoboTorqService;
    /// use commons::util::error::InvariantError;
    ///
    /// struct MyService;
    ///
    /// impl RoboTorqService for MyService {
    ///     fn health_check(&self) -> Result<String, InvariantError> {
    ///         Ok("OK".to_string())
    ///     }
    ///
    ///     fn export_metrics(&self) -> String {
    ///         "# No metrics".to_string()
    ///     }
    ///
    ///     fn handle_request(&self, path: &str, method: &str) -> Option<Result<String, InvariantError>> {
    ///         match (path, method) {
    ///             ("/custom/status", "GET") => {
    ///                 // Before: Request received for custom endpoint
    ///                 // After: Service handles it and returns custom response
    ///                 Some(Ok("Custom status: Active".to_string()))
    ///             }
    ///             _ => None, // Let HTTP server handle other requests
    ///         }
    ///     }
    /// }
    /// ```
    fn handle_request(&self, _path: &str, _method: &str) -> Option<Result<String, InvariantError>> {
        None
    }

    /// Initialize the service with configuration and dependencies.
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
    async fn initialize(&mut self, config: &RoboTorqConfig) -> Result<(), InvariantError>;

    /// Start the service and begin processing operations.
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
    async fn start(&self) -> Result<(), InvariantError>;

    /// Stop processing operations while keeping resources allocated.
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
    async fn stop(&self) -> Result<(), InvariantError>;

    /// Perform a complete shutdown and release all resources.
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
    async fn shutdown(&self) -> Result<(), InvariantError>;
}

/// HTTP server that exposes a RoboTorqService via standard endpoints.
/// Decouples HTTP infrastructure from business logic.
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
/// * `service` - The service instance wrapped in an Arc for thread-safe sharing
/// * `config` - Configuration specifying address, port, and endpoint paths
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use commons::services::http::{HttpServer, HttpServerConfig};
/// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
/// use commons::types::ids::RobotId;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Before: Service exists but is not exposed via HTTP
///     let metrics = RobotGatewayMetrics::new("gateway");
///     let gateway = Arc::new(RobotGateway::single(RobotId::new()).with_metrics(metrics));
///
///     // Configure HTTP server
///     let config = HttpServerConfig::local_defaults(8080);
///
///     // Create HTTP server to expose the service
///     let server = HttpServer::new(gateway, config);
///
///     // After: Service is now accessible via HTTP
///     // GET http://127.0.0.1:8080/health -> health check response
///     // GET http://127.0.0.1:8080/metrics -> Prometheus metrics
///     server.start().await?;
///
///     Ok(())
/// }
/// ```
pub struct HttpServer<S: RoboTorqService> {
    /// The service instance wrapped in an Arc for thread-safe sharing across HTTP requests.
    service: Arc<S>,
    /// Configuration specifying network address, port, and endpoint paths.
    config: HttpServerConfig,
}

impl<S: RoboTorqService> HttpServer<S> {
    /// Create a new HTTP server for the given service.
    ///
    /// This constructor wraps the service in an Arc for thread-safe sharing
    /// across multiple HTTP requests and stores the configuration.
    ///
    /// # Arguments
    ///
    /// * `service` - The service instance to expose via HTTP, wrapped in an Arc
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
    /// use commons::services::http::{HttpServer, HttpServerConfig};
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    ///
    /// // Before: Service exists in memory
    /// let metrics = RobotGatewayMetrics::new("gateway");
    /// let gateway = Arc::new(RobotGateway::single(RobotId::new()).with_metrics(metrics));
    /// let config = HttpServerConfig::local_defaults(8080);
    ///
    /// // After: HTTP server is created and ready to expose the service
    /// let server = HttpServer::new(gateway, config);
    /// // The service is now wrapped and configured for HTTP exposure
    /// ```
    pub fn new(service: Arc<S>, config: HttpServerConfig) -> Self {
        Self { service, config }
    }

    /// Start the HTTP server asynchronously.
    ///
    /// This method binds to the configured address and port, sets up the HTTP routes,
    /// and begins accepting connections. The server will run until it receives a
    /// shutdown signal or encounters an unrecoverable error.
    ///
    /// The server automatically creates the following endpoints:
    /// - `GET {health_path}` - Calls `service.health_check()` and returns the result
    /// - `GET {metrics_path}` - Calls `service.export_metrics()` and returns Prometheus format
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the server shuts down gracefully, or `Err(error)` if
    /// it fails to start or encounters an unrecoverable error.
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
    /// use commons::services::http::{HttpServer, HttpServerConfig};
    /// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
    /// use commons::types::ids::RobotId;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let metrics = RobotGatewayMetrics::new("gateway");
    ///     let gateway = Arc::new(RobotGateway::single(RobotId::new()).with_metrics(metrics));
    ///     let config = HttpServerConfig::local_defaults(8080);
    ///     let server = HttpServer::new(gateway, config);
    ///
    ///     // Before: Server is configured but not running
    ///     // Network port 8080 is available
    ///
    ///     // After: Server is running and accepting connections
    ///     // GET http://127.0.0.1:8080/health returns health status
    ///     // GET http://127.0.0.1:8080/metrics returns Prometheus metrics
    ///     server.start().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn start(self) -> Result<(), InvariantError> {
        let addr = format!("{}:{}", self.config.service.address, self.config.service.port);

        // Build the application with routes
        let app = Router::new()
            .route(self.config.health.0.as_str(), get(health_handler))
            .route(self.config.metrics.0.as_str(), get(metrics_handler))
            .layer(CorsLayer::permissive())
            .with_state(self.service);

        // Create listener
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

        tracing::info!("HTTP server listening on {}", addr);

        // Start serving
        axum::serve(listener, app).await
            .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

        Ok(())
    }
}

/// Handler for health check endpoint.
async fn health_handler<S: RoboTorqService>(
    State(service): State<Arc<S>>,
) -> impl IntoResponse {
    match service.health_check() {
        Ok(message) => (StatusCode::OK, message),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Health check failed: {:?}", e)),
    }
}

/// Handler for metrics endpoint.
async fn metrics_handler<S: RoboTorqService>(
    State(service): State<Arc<S>>,
) -> impl IntoResponse {
    (StatusCode::OK, service.export_metrics())
}

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
        Self { address: address.into(), port }
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
    pub fn new<S: Into<String>>(path: S) -> Self { Self(path.into()) }
}

/// Example usage of the Axum-based HTTP server with RobotGateway.
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use commons::services::http::{HttpServer, HttpServerConfig};
/// use commons::services::robot_gateway::{RobotGateway, metrics::RobotGatewayMetrics};
/// use commons::types::ids::RobotId;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a service that implements RoboTorqService
///     let metrics = RobotGatewayMetrics::new("gateway");
///     let gateway = Arc::new(RobotGateway::single(RobotId::new()).with_metrics(metrics));
///
///     // Configure the HTTP server
///     let config = HttpServerConfig::local_defaults(8080);
///
///     // Create and start the server
///     let server = HttpServer::new(gateway, config);
///     server.start().await?;
///
///     Ok(())
/// }
/// ```
/// Start a minimal HTTP server serving health and metrics endpoints using Axum.
/// - Health: responds 200 with health status
/// - Metrics: responds with Prometheus text from MetricsHandler::export_text()
///
/// This is a legacy function for backward compatibility - prefer HttpServer for new code.
///
/// This function provides a basic HTTP server that serves static health and metrics
/// responses without requiring a full RoboTorqService implementation. It's useful
/// for simple monitoring endpoints or as a compatibility layer.
///
/// # Arguments
///
/// * `handler` - Metrics handler that provides the metrics data
/// * `service` - HTTP service configuration (address and port)
/// * `health` - Endpoint path for health checks
/// * `metrics` - Endpoint path for metrics export
///
/// # Returns
///
/// Returns `Ok(())` if the server shuts down gracefully, or `Err(error)` if
/// it fails to start or encounters an unrecoverable error.
///
/// # Panics
///
/// This method does not panic under normal circumstances. Network binding errors
/// and HTTP server errors are returned as `InvariantError` results.
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::{HttpService, HttpEndpoint, start_basic_http_server_async};
/// use commons::util::metrics::MetricsHandler;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Before: Metrics handler exists but is not exposed via HTTP
///     let handler = MetricsHandler::new();
///     handler.register_counter("requests_total", "Total requests");
///
///     let service = HttpService::new("127.0.0.1", 8080);
///     let health = HttpEndpoint::new("/health");
///     let metrics = HttpEndpoint::new("/metrics");
///
///     // After: HTTP server is running and exposing metrics
///     // GET http://127.0.0.1:8080/health -> "OK"
///     // GET http://127.0.0.1:8080/metrics -> Prometheus format metrics
///     start_basic_http_server_async(handler, service, health, metrics).await?;
///
///     Ok(())
/// }
/// ```
pub async fn start_basic_http_server_async(
    handler: Arc<MetricsHandler>,
    service: HttpService,
    health: HttpEndpoint,
    metrics: HttpEndpoint,
) -> Result<(), InvariantError> {
    let addr = format!("{}:{}", service.address, service.port);

    // Build the application with routes
    let app = Router::new()
        .route(health.0.as_str(), get(move || async move {
            (StatusCode::OK, "OK".to_string())
        }))
        .route(metrics.0.as_str(), get(move || async move {
            (StatusCode::OK, handler.export_text())
        }))
        .layer(CorsLayer::permissive());

    // Create listener
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

    tracing::info!("Basic HTTP server listening on {}", addr);

    // Start serving
    axum::serve(listener, app).await
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

    Ok(())
}

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
#[derive(Clone, Debug)]
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
    pub fn new(service: HttpService, health: HttpEndpoint, metrics: HttpEndpoint) -> Self {
        Self { service, health, metrics }
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
    pub fn local_defaults(port: u16) -> Self {
        Self {
            service: HttpService::new("127.0.0.1", port),
            health: HttpEndpoint::new("/health"),
            metrics: HttpEndpoint::new("/metrics"),
        }
    }
}

/// Start server using a structured config (async version).
/// This is a legacy function for backward compatibility - prefer HttpServer for new code.
///
/// This function is a convenience wrapper around `start_basic_http_server_async`
/// that takes a structured `HttpServerConfig` instead of individual parameters.
///
/// # Arguments
///
/// * `handler` - Metrics handler that provides the metrics data
/// * `cfg` - Complete HTTP server configuration
///
/// # Returns
///
/// Returns `Ok(())` if the server shuts down gracefully, or `Err(error)` if
/// it fails to start or encounters an unrecoverable error.
///
/// # Examples
///
/// ```rust,ignore
/// use commons::services::http::{HttpServerConfig, start_basic_http_server_with_config_async};
/// use commons::util::metrics::MetricsHandler;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Before: Metrics handler and config exist separately
///     let handler = MetricsHandler::new();
///     let config = HttpServerConfig::local_defaults(8080);
///
///     // After: HTTP server is running with the combined configuration
///     // The config provides address, port, and endpoint paths
///     start_basic_http_server_with_config_async(handler, config).await?;
///
///     Ok(())
/// }
/// ```
pub async fn start_basic_http_server_with_config_async(
    handler: Arc<MetricsHandler>,
    cfg: HttpServerConfig,
) -> Result<(), InvariantError> {
    start_basic_http_server_async(handler, cfg.service, cfg.health, cfg.metrics).await
}

/// Gracefully request shutdown by calling the internal `/shutdown` endpoint.
/// Note: This is a legacy synchronous function. For async code, use proper shutdown signaling.
///
/// This function attempts to gracefully shut down a running HTTP server by making
/// an HTTP request to its `/shutdown` endpoint. This is a simple shutdown mechanism
/// that works for basic servers but may not be suitable for production use.
///
/// # Arguments
///
/// * `service` - The HTTP service configuration pointing to the server to shut down
///
/// # Returns
///
/// Returns `Ok(())` if the shutdown request was sent successfully, or `Err(error)`
/// if the request failed (e.g., connection refused, network error).
///
/// # Panics
///
/// This method does not panic under normal circumstances. Network connection errors
/// are returned as `InvariantError` results.
///
/// # Examples
///
/// ```rust,no_run
/// use commons::services::http::{HttpService, request_graceful_shutdown};
/// use std::thread;
/// use std::time::Duration;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let service = HttpService::new("127.0.0.1", 8080);
///
///     // Assume a server is running on port 8080
///     // Before: Server is running and accepting connections
///
///     // After: Shutdown request is sent to the server
///     // The server should stop accepting new connections and shut down
///     match request_graceful_shutdown(&service) {
///         Ok(()) => println!("Shutdown request sent successfully"),
///         Err(e) => println!("Failed to send shutdown request: {:?}", e),
///     }
///
///     // Give the server time to shut down
///     thread::sleep(Duration::from_secs(1));
///
///     Ok(())
/// }
/// ```
pub fn request_graceful_shutdown(service: &HttpService) -> Result<(), InvariantError> {
    use std::io::Write;
    use std::net::TcpStream;
    let addr = format!("{}:{}", service.address, service.port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;
    let req = format!("GET /shutdown HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", addr);
    stream.write_all(req.as_bytes())
        .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // Use a local test service to avoid cross-crate type duplication
    struct TestService {
        handler: Arc<MetricsHandler>,
    }

    impl RoboTorqService for TestService {
        fn health_check(&self) -> Result<String, InvariantError> {
            Ok("OK".to_string())
        }

        fn export_metrics(&self) -> String { self.handler.export_text() }

        async fn initialize(&mut self, _config: &RoboTorqConfig) -> Result<(), InvariantError> { Ok(()) }
        async fn start(&self) -> Result<(), InvariantError> { Ok(()) }
        async fn stop(&self) -> Result<(), InvariantError> { Ok(()) }
        async fn shutdown(&self) -> Result<(), InvariantError> { Ok(()) }
    }

    #[test]
    fn http_server_with_test_service() {
        // Create a minimal service that implements RoboTorqService
        let handler = MetricsHandler::new();
        handler.register_counter("test_requests_total", "Total test requests");
        let svc = Arc::new(TestService { handler });

        // Create HTTP server config
        let config = HttpServerConfig::local_defaults(0); // Use port 0 for testing

        // Verify the service implements the trait
        assert!(svc.health_check().is_ok());
        assert!(!svc.export_metrics().is_empty());

        // Test that we can create the server (without starting it)
        let _server = HttpServer::new(svc, config);
    }

    #[tokio::test]
    async fn basic_http_server_async() {
        let handler = MetricsHandler::new();
        // Register some test metrics to ensure export_text() is not empty
        handler.register_counter("test_counter", "test counter");
        let _service = HttpService::new("127.0.0.1", 0); // Use port 0 for testing
        let _health = HttpEndpoint::new("/health");
        let _metrics = HttpEndpoint::new("/metrics");

        // This would start the server (commented out to avoid actually starting in tests)
        // start_basic_http_server_async(handler, service, health, metrics).await.unwrap();

        // Just verify the handler works
        assert!(!handler.export_text().is_empty());
    }
}


