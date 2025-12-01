//! Axum HTTP utilities for exposing `robotorq_service` implementations.
//!
//! Handlers and lifecycle helpers live in focused submodules: `healthz`,
//! `readyz`, `metrics`, `initialization`, and `shutdown`. Prefer those over
//! adding logic here.
#![allow(async_fn_in_trait)]
use crate::util::config::RoboTorqConfig;
use crate::util::error::{InvariantError, logging_error::LoggingError};
use axum::{Extension, Router, routing::get};
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
pub mod service_metrics;
pub mod middleware;
pub mod readyz;
pub mod shutdown;
pub use healthz::health_handler;
pub use initialization::{initialize_service, load_and_initialize_service};
pub use service_metrics::metrics_handler;
pub use shutdown::{ctrl_c_signal, ctrl_c_signal_with_service_shutdown};
pub mod service_metrics_context;
pub mod label_source;
pub use label_source::{build_label_set, LabelSet};

/// Core trait for services exposed via standardized HTTP endpoints.
///
/// Intentionally uses snake_case naming to align with RoboTorq's
/// internal service taxonomy and avoid conflating trait names with
/// concrete service structs. Clippy's `non_camel_case_types` lint
/// is explicitly allowed here.
#[allow(non_camel_case_types)]
pub trait robotorq_service: Send + Sync + 'static {
    /// Liveness check; `GET /healthz` returns 200 when this is Ok.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if the service is not healthy or encounters
    /// an error during the health check. The specific error depends on the
    /// service implementation.
    /// Default implementation returns a simple "ok" status.
    /// Services should override to perform real dependency checks.
    fn health_check(&self) -> Result<String, InvariantError> {
        Ok("ok".to_string())
    }

    /// Export metrics in Prometheus text format; served at `/metrics`.
    /// Default implementation returns an empty metrics payload.
    /// Override to export Prometheus-formatted metrics.
    fn export_metrics(&self) -> String {
        String::new()
    }

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
    /// ```rust,no_run
    /// use commons::services::robotorq_service::robotorq_service;
    /// use commons::util::config::load_robotorq_config;
    /// use commons::util::error::InvariantError;
    /// struct MySvc;
    /// impl robotorq_service for MySvc {}
    /// #[tokio::main]
    /// async fn main() -> Result<(), InvariantError> {
    ///     let mut svc = MySvc;
    ///     let cfg = load_robotorq_config(None).unwrap();
    ///     svc.initialize(&cfg).await?;
    ///     Ok(())
    /// }
    /// ```
    async fn initialize(&mut self, _config: &RoboTorqConfig) -> Result<(), InvariantError> {
        Ok(())
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
    /// ```rust,no_run
    /// use commons::services::robotorq_service::robotorq_service;
    /// use commons::util::config::load_robotorq_config;
    /// use commons::util::error::InvariantError;
    /// struct MySvc;
    /// impl robotorq_service for MySvc {}
    /// #[tokio::main]
    /// async fn main() -> Result<(), InvariantError> {
    ///     let mut svc = MySvc;
    ///     let cfg = load_robotorq_config(None).unwrap();
    ///     svc.initialize(&cfg).await?;
    ///     svc.start().await?;
    ///     Ok(())
    /// }
    /// ```
    async fn start(&self) -> Result<(), InvariantError> {
        Ok(())
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
    /// ```rust,no_run
    /// use commons::services::robotorq_service::robotorq_service;
    /// use commons::util::error::InvariantError;
    /// struct MySvc;
    /// impl robotorq_service for MySvc {}
    /// #[tokio::main]
    /// async fn main() -> Result<(), InvariantError> {
    ///     let svc = MySvc;
    ///     svc.start().await?;
    ///     svc.stop().await?;
    ///     Ok(())
    /// }
    /// ```
    async fn stop(&self) -> Result<(), InvariantError> {
        Ok(())
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
    /// ```rust,no_run
    /// use commons::services::robotorq_service::robotorq_service;
    /// use commons::util::error::InvariantError;
    /// struct MySvc;
    /// impl robotorq_service for MySvc {}
    /// #[tokio::main]
    /// async fn main() -> Result<(), InvariantError> {
    ///     let svc = MySvc;
    ///     svc.shutdown().await?;
    ///     Ok(())
    /// }
    /// ```
    async fn shutdown(&self) -> Result<(), InvariantError> {
        Ok(())
    }
}

/// Optional metrics context hook for services.
///
/// Services can implement this to receive a `ServiceMetricsContext` constructed
/// by the HTTP server when a metrics registry is configured. Default is no-op.
pub trait RoboTorqServiceMetricsExt {
    /// Inject a shared `ServiceMetricsContext` created by the HTTP server.
    ///
    /// Services can store this context to emit standardized lifecycle metrics.
    /// Default implementation is a no-op, so adoption is opt-in.
    fn set_metrics_context(&mut self, _ctx: Option<std::sync::Arc<service_metrics_context::ServiceMetricsContext>>) {}
}

impl<T: robotorq_service> RoboTorqServiceMetricsExt for T {}

/// Lightweight Axum server exposing standardized endpoints for a service.
///
/// The HttpServer automatically creates HTTP endpoints for any service that
/// implements `robotorq_service`. It provides standard `/health` and `/metrics`
/// endpoints, CORS support, and proper error handling.
///
/// # Type Parameters
///
/// * `S` - The service type that implements `robotorq_service`
///
/// # Fields
///
/// * `service` - The service instance wrapped in an Arc<Mutex> for thread-safe mutable access
/// * `config` - Configuration specifying address, port, and endpoint paths
///
/// # Examples
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, robotorq_service};
/// use commons::util::config::load_robotorq_config;
/// use commons::util::error::InvariantError;
/// struct MySvc;
/// impl robotorq_service for MySvc {}
/// #[tokio::main]
/// async fn main() -> Result<(), InvariantError> {
///     let svc = Arc::new(Mutex::new(MySvc));
///     let server = HttpServer::new(svc, HttpServerConfig::local_defaults(0));
///     let cfg = load_robotorq_config(None).unwrap();
///     // Normally: server.start(&cfg).await?; (omit to avoid binding a port during doc test)
///     Ok(())
/// }
/// ```
pub struct HttpServer<S: robotorq_service> {
    /// The service instance wrapped in an Arc<Mutex> for thread-safe mutable access across HTTP requests.
    service: Arc<Mutex<S>>,
    /// Configuration specifying network address, port, and endpoint paths.
    config: HttpServerConfig,
    /// Readiness flag indicating whether the server is ready to serve traffic.
    ready: Arc<AtomicBool>,
    // Optional metrics registry for middleware; can be None in minimal setups
    // (Will be extended in Phase 1 wiring.)
}

impl<S: robotorq_service> HttpServer<S> {
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
        /// ```rust,no_run
        /// use std::sync::Arc;
        /// use tokio::sync::Mutex;
        /// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, robotorq_service};
        /// struct MySvc; impl robotorq_service for MySvc {}
        /// let server = HttpServer::new(Arc::new(Mutex::new(MySvc)), HttpServerConfig::local_defaults(0));
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
        /// ```rust,no_run
        /// use std::sync::Arc;
        /// use tokio::sync::Mutex;
        /// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, robotorq_service};
        /// use commons::util::config::load_robotorq_config;
        /// use commons::util::error::InvariantError;
        /// struct MySvc; impl robotorq_service for MySvc {}
        /// #[tokio::main]
        /// async fn main() -> Result<(), InvariantError> {
        ///     let http_config = HttpServerConfig::local_defaults(0);
        ///     let cfg = load_robotorq_config(None).unwrap();
        ///     let server = HttpServer::new(Arc::new(Mutex::new(MySvc)), http_config);
        ///     // server.start(&cfg).await?;  // omitted to keep doc test fast
        ///     Ok(())
        /// }
        /// ```
    pub async fn start(mut self, config: &RoboTorqConfig) -> Result<(), InvariantError> {
        // Initialize the service
        {
            let mut service = self.service.lock().await;
            // Ensure a metrics registry exists; create one with config-derived labels if missing
            let labels = build_label_set(config);
            let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = if let Some(r) = &self.config.metrics_registry {
                std::sync::Arc::clone(r)
            } else {
                let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                    &labels.service,
                    &labels.component,
                    &labels.version,
                ));
                self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                r
            };
            let ctx = service_metrics_context::ServiceMetricsContext::new(std::sync::Arc::clone(&registry_arc), labels);
            service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
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
        let shutdown_signal = async move { shutdown::ctrl_c_signal().await };

        // Start HTTP server with shutdown
        self.start_with_shutdown(shutdown_signal).await
    }

    /// Bind, route, and serve using config auto-loaded inside commons.
    ///
    /// This convenience method keeps callers clean by loading `RoboTorqConfig`
    /// within commons and driving the full lifecycle. Errors during
    /// initialization are logged and the server continues, per policy.
    pub async fn start_autoload(mut self) -> Result<(), InvariantError> {
        // Initialize with autoloaded config
        {
            let mut service = self.service.lock().await;
            match load_and_initialize_service(&mut *service).await {
                Ok(cfg) => {
                    let labels = build_label_set(&cfg);
                    let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = if let Some(r) = &self.config.metrics_registry {
                        std::sync::Arc::clone(r)
                    } else {
                        let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                            &labels.service,
                            &labels.component,
                            &labels.version,
                        ));
                        self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                        r
                    };
                    let ctx = service_metrics_context::ServiceMetricsContext::new(std::sync::Arc::clone(&registry_arc), labels);
                    service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "service initialize failed, continuing");
                    // Build minimal labels and registry in failure path as well
                    let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
                        crate::util::config::RoboTorqConfig {
                            schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                            mode: crate::util::config::Mode::Production,
                            simulation: Default::default(),
                            ports: crate::util::config::load_ports_config_from_default(),
                            http: Default::default(),
                            nats: Default::default(),
                            persistence: Default::default(),
                            observability: Default::default(),
                            security: Default::default(),
                            crypto: Default::default(),
                            economic: Default::default(),
                        }
                    });
                    let labels = build_label_set(&cfg);
                    let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = if let Some(r) = &self.config.metrics_registry {
                        std::sync::Arc::clone(r)
                    } else {
                        let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                            &labels.service,
                            &labels.component,
                            &labels.version,
                        ));
                        self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                        r
                    };
                    let ctx = service_metrics_context::ServiceMetricsContext::new(std::sync::Arc::clone(&registry_arc), labels);
                    service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
                }
            }
        }

        // Start the service
        {
            let service = self.service.lock().await;
            service.start().await?;
        }

        // Create shutdown signal that will call stop and shutdown
        let shutdown_signal = async move { shutdown::ctrl_c_signal().await };

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
            .route(self.config.metrics.0.as_str(), get(metrics_handler::<S>))
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
            app = app.route_layer(middleware::HttpMetricsLayer::new(std::sync::Arc::clone(
                registry,
            )));
            // Attach HTTP endpoint-specific metrics (e.g., /healthz)
            let healthz_metrics = std::sync::Arc::new(healthz::HealthzMetrics::new(std::sync::Arc::clone(registry)));
            app = app.layer(Extension(healthz_metrics));
        }

        // Create listener
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| InvariantError::Logging(LoggingError::from(e.to_string())))?;

        tracing::info!("HTTP server listening on {}", addr);

        // Mark ready and start serving; export readiness gauge if registry present
        let mut ready_gauge: Option<Arc<dyn crate::util::metrics::MetricGauge + Send + Sync>> =
            None;
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

        // Invoke graceful cleanup hooks after server stops (Ctrl+C or error)
        {
            let service = self.service.lock().await;
            if let Err(err) = service.stop().await {
                tracing::warn!(error = ?err, "service stop hook failed");
            }
            if let Err(err) = service.shutdown().await {
                tracing::warn!(error = ?err, "service shutdown hook failed");
            }
        }

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
/// ```rust,no_run
/// use commons::services::robotorq_service::HttpService;
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
    /// ```rust,no_run
    /// use commons::services::robotorq_service::HttpService;
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
/// ```rust,no_run
/// use commons::services::robotorq_service::HttpEndpoint;
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
    /// use commons::services::robotorq_service::HttpEndpoint;
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
/// use commons::services::robotorq_service::{HttpServerConfig, HttpService, HttpEndpoint};
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
    /// use commons::services::robotorq_service::{HttpServerConfig, HttpService, HttpEndpoint};
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
    /// use commons::services::robotorq_service::HttpServerConfig;
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

    impl robotorq_service for TestService {
        fn health_check(&self) -> Result<String, InvariantError> {
            Ok("OK".to_string())
        }

        fn export_metrics(&self) -> String {
            self.handler.export_text()
        }

        async fn initialize(&mut self, _config: &RoboTorqConfig) -> Result<(), InvariantError> {
            Ok(())
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

    #[test]
    fn http_server_with_test_service() {
        // Create a minimal service that implements RoboTorqService
        let handler = MetricsHandler::new();
        let _ = handler.register_counter("test_requests_total", "Total test requests");
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
        let _ = handler.register_counter("test_counter", "test counter");
        let _service = HttpService::new("127.0.0.1", 0); // Use port 0 for testing
        let _health = HttpEndpoint::new("/health");
        let _metrics = HttpEndpoint::new("/metrics");

        // Just verify the handler works
        assert!(!handler.export_text().is_empty());
    }
}
