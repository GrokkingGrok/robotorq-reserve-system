//! Axum HTTP utilities for exposing `robotorq_service` implementations.
//!
//! Handlers and lifecycle helpers live in focused submodules: `healthz`,
//! `readyz`, `metrics`, `initialization`, and `shutdown`. Prefer those over
//! adding logic here.
#![allow(async_fn_in_trait)]
use crate::util::config::RoboTorqConfig;
use crate::util::error::ServiceError;
use axum::{Extension, Router, routing::get};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

/// Marker trait indicating a type participates in the RoboTorq service lifecycle.
///
/// Implementors typically also implement the `RoboTorqService` facade trait which
/// provides default async lifecycle hooks (`initialize`, `start`, `stop`, `shutdown`).
/// This marker enables future decoupling (e.g., composing lifecycle-only adapters)
/// without forcing all downstream code to depend on metric or health capabilities.
pub trait ServiceLifecycle {}

// Submodules wiring and re-exports
pub mod healthz;
mod initialization;
pub mod middleware;
pub mod readyz;
pub mod service_metrics;
pub mod shutdown;
pub use healthz::health_handler;
pub use initialization::{initialize_service, load_and_initialize_service};
pub use service_metrics::metrics_handler;
pub use shutdown::{ctrl_c_signal, ctrl_c_signal_with_service_shutdown};
pub mod label_source;
pub mod service_metrics_context;
pub use label_source::{LabelSet, MetricsLabelProvider, build_label_set};

/// Contributes to health-check endpoints.
/// Contributes auxiliary health status details beyond simple liveness.
///
/// The `RoboTorqService::health_check` method returns a `Result<String, ServiceError>`
/// for liveness gating; `HealthContributor` supplies a lightweight, fallible‐free
/// string snapshot (e.g. dependency summary) that can be merged into richer endpoints
/// in later phases. For now it is a simple extension point.
pub trait HealthContributor {
    /// Return a human‑readable summary of auxiliary health state.
    fn health_status(&self) -> String;
}

/// Manages metrics context and exports metrics.
/// Contributes metrics emission and receives an optional shared metrics context.
///
/// Services can opt-in by overriding `set_metrics_context` to retain the context
/// for standardized counter/gauge construction. If they export service-local
/// metrics they override `export_metrics` to return Prometheus exposition text.
pub trait MetricsContributor {
    /// Inject a shared metrics context established by `HttpServer` metric wiring.
    fn set_metrics_context(
        &mut self,
        _ctx: Option<std::sync::Arc<service_metrics_context::ServiceMetricsContext>>,
    ) {
    }
    /// Export Prometheus text metrics specific to the service implementation.
    fn export_metrics(&self) -> String {
        String::new()
    }
}

// Update RoboTorqService to compose the smaller traits
/// RoboTorq facade trait combining lifecycle, health, and metrics behaviors with defaults.
///
/// Implement this trait for each service entry point. Override only what you need:
/// - `initialize` to allocate dependencies (DB pools, NATS clients, etc.)
/// - `start` to begin active processing (spawn tasks, subscribe streams)
/// - `stop` for graceful quiescing (stop intake, flush work)
/// - `shutdown` for final resource release.
/// - `health_check` to surface dependency status.
/// - `export_metrics` to expose service-local Prometheus metrics.
///
/// All methods provide no-op / healthy defaults to minimize boilerplate for simple services.
pub trait RoboTorqService: ServiceLifecycle + HealthContributor + MetricsContributor {
    /// Liveness check used by `/healthz`.
    fn health_check(&self) -> Result<String, ServiceError> {
        Ok("ok".to_string())
    }
    /// Export Prometheus metrics (service-local portion). Override for real metrics.
    fn export_metrics(&self) -> String {
        String::new()
    }
    /// Allocate dependencies & prepare resources. Override for initialization logic.
    async fn initialize(&mut self, _cfg: &RoboTorqConfig) -> Result<(), ServiceError> {
        Ok(())
    }
    /// Transition to active processing (spawn tasks, subscribe). Override as needed.
    async fn start(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    /// Graceful pause of new work; finalize in-flight requests. Override for quiesce behavior.
    async fn stop(&self) -> Result<(), ServiceError> {
        Ok(())
    }
    /// Final cleanup releasing all resources. Override for teardown.
    async fn shutdown(&self) -> Result<(), ServiceError> {
        Ok(())
    }
}

// Note: No blanket impl for `RoboTorqService` to avoid conflicts with explicit impls in services.

// Backwards-compatible alias for existing code/tests using `robotorq_service`.
// Note: legacy alias `robotorq_service` removed. Use `RoboTorqService` directly.

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
/// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, RoboTorqService};
/// use commons::util::config::load_robotorq_config;
/// use commons::util::error::ServiceError;
/// struct MySvc;
/// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
/// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".to_string() } }
/// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
/// impl RoboTorqService for MySvc {}
/// #[tokio::main]
/// async fn main() -> Result<(), ServiceError> {
///     let svc = Arc::new(Mutex::new(MySvc));
///     let server = HttpServer::new(svc, HttpServerConfig::local_defaults(0));
///     let cfg = load_robotorq_config(None).unwrap();
///     // Normally: server.start(&cfg).await?; (omit to avoid binding a port during doc test)
///     Ok(())
/// }
/// ```
pub struct HttpServer<S: RoboTorqService + Send + Sync + 'static> {
    /// The service instance wrapped in an Arc<Mutex> for thread-safe mutable access across HTTP requests.
    service: Arc<Mutex<S>>,
    /// Configuration specifying network address, port, and endpoint paths.
    config: HttpServerConfig,
    /// Readiness flag indicating whether the server is ready to serve traffic.
    ready: Arc<AtomicBool>,
    // Optional metrics registry for middleware; can be None in minimal setups
    // (Will be extended in Phase 1 wiring.)
}

impl<S: RoboTorqService + Send + Sync + 'static> HttpServer<S> {
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
    /// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, RoboTorqService};
    /// struct MySvc;
    /// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
    /// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".to_string() } }
    /// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
    /// impl RoboTorqService for MySvc {}
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
    /// Returns `ServiceError` if the server fails to bind to the configured
    /// address and port, or if the HTTP server encounters an unrecoverable error.
    ///
    /// # Panics
    ///
    /// This method does not panic under normal circumstances. Network binding errors
    /// and HTTP server errors are returned as `ServiceError` results.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    /// use commons::services::robotorq_service::{HttpServer, HttpServerConfig, RoboTorqService};
    /// use commons::util::config::load_robotorq_config;
    /// use commons::util::error::ServiceError;
    /// struct MySvc;
    /// impl commons::services::robotorq_service::ServiceLifecycle for MySvc {}
    /// impl commons::services::robotorq_service::HealthContributor for MySvc { fn health_status(&self) -> String { "OK".to_string() } }
    /// impl commons::services::robotorq_service::MetricsContributor for MySvc {}
    /// impl RoboTorqService for MySvc {}
    /// #[tokio::main]
    /// async fn main() -> Result<(), ServiceError> {
    ///     let http_config = HttpServerConfig::local_defaults(0);
    ///     let cfg = load_robotorq_config(None).unwrap();
    ///     let server = HttpServer::new(Arc::new(Mutex::new(MySvc)), http_config);
    ///     // server.start(&cfg).await?;  // omitted to keep doc test fast
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument(skip(self, config))]
    pub async fn start(mut self, config: &RoboTorqConfig) -> Result<(), ServiceError> {
        // Initialize the service
        {
            let mut service = self.service.lock().await;
            // Ensure a metrics registry exists; create one with provided labels if missing
            let labels = if let Some(provider) = &self.config.label_provider {
                let l = provider.labels();
                tracing::debug!(service=%l.service, component=%l.component, version=%l.version, "using provided static label set");
                l
            } else {
                let l = build_label_set(config);
                tracing::debug!(service=%l.service, component=%l.component, version=%l.version, "derived label set from config");
                l
            };
            let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                if let Some(r) = &self.config.metrics_registry {
                    std::sync::Arc::clone(r)
                } else {
                    tracing::debug!(service=%labels.service, component=%labels.component, "creating new PrometheusRegistry for HTTP server");
                    let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                        std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                            &labels.service,
                            &labels.component,
                            &labels.version,
                        ));
                    tracing::info!(service=%labels.service, component=%labels.component, "created PrometheusRegistry instance");
                    self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                    r
                };
            let ctx = service_metrics_context::ServiceMetricsContext::new(
                std::sync::Arc::clone(&registry_arc),
                labels.clone(),
            );
            tracing::info!(service=%labels.service, component=%labels.component, version=%labels.version, "metrics registry attached");
            tracing::debug!(service=%labels.service, component=%labels.component, "metrics context created and attached to service");
            service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
            if let Err(err) = service.initialize(config).await {
                tracing::warn!(error = ?err, "service initialize failed, continuing");
            }
        }

        // Start the service
        {
            let service = self.service.lock().await;
            tracing::info!("invoking service.start()");
            let start_ts = std::time::Instant::now();
            service
                .start()
                .await
                .map_err(|e| ServiceError::Other(format!("service.start failed: {:?}", e)))?;
            tracing::info!(duration_ms=%start_ts.elapsed().as_millis(), "service.start() completed");
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
    #[tracing::instrument(skip(self))]
    pub async fn start_autoload(mut self) -> Result<(), ServiceError> {
        // Initialize with autoloaded config
        {
            let mut service = self.service.lock().await;
            match load_and_initialize_service(&mut *service).await {
                Ok(cfg) => {
                    let labels = if let Some(provider) = &self.config.label_provider {
                        let l = provider.labels();
                        tracing::debug!(service=%l.service, component=%l.component, version=%l.version, "using provided static label set (autoload)");
                        l
                    } else {
                        let l = build_label_set(&cfg);
                        tracing::debug!(service=%l.service, component=%l.component, version=%l.version, "derived label set from config (autoload)");
                        l
                    };
                    let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                        if let Some(r) = &self.config.metrics_registry {
                            std::sync::Arc::clone(r)
                        } else {
                            let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                                std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                                    &labels.service,
                                    &labels.component,
                                    &labels.version,
                                ));
                            self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                            r
                        };
                    let ctx = service_metrics_context::ServiceMetricsContext::new(
                        std::sync::Arc::clone(&registry_arc),
                        labels,
                    );
                    service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
                }
                Err(err) => {
                    tracing::warn!(error = ?err, "service initialize failed, continuing");
                    // Build minimal labels and registry in failure path as well
                    let cfg =
                        crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
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
                    let registry_arc: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                        if let Some(r) = &self.config.metrics_registry {
                            std::sync::Arc::clone(r)
                        } else {
                            let r: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
                                std::sync::Arc::new(crate::util::metrics::PrometheusRegistry::new(
                                    &labels.service,
                                    &labels.component,
                                    &labels.version,
                                ));
                            self.config.metrics_registry = Some(std::sync::Arc::clone(&r));
                            r
                        };
                    let ctx = service_metrics_context::ServiceMetricsContext::new(
                        std::sync::Arc::clone(&registry_arc),
                        labels,
                    );
                    service.set_metrics_context(Some(std::sync::Arc::new(ctx)));
                }
            }
        }

        // Start the service
        {
            let service = self.service.lock().await;
            tracing::info!("invoking service.start() (autoload)");
            let start_ts = std::time::Instant::now();
            service
                .start()
                .await
                .map_err(|e| ServiceError::Other(format!("service.start failed: {:?}", e)))?;
            tracing::info!(duration_ms=%start_ts.elapsed().as_millis(), "service.start() completed (autoload)");
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
    #[tracing::instrument(skip(self, shutdown_signal))]
    pub async fn start_with_shutdown<F>(self, shutdown_signal: F) -> Result<(), ServiceError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr = format!(
            "{}:{}",
            self.config.service.address, self.config.service.port
        );

        let server_span = tracing::span!(tracing::Level::INFO, "http_server", addr = %addr);
        let _server_enter = server_span.enter();

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
            tracing::debug!(body_limit = body_limit, "enabling request body limit layer");
            app = app.layer(RequestBodyLimitLayer::new(body_limit));
        }

        if let Some(timeout_secs) = self.config.timeout_seconds {
            tracing::debug!(timeout = timeout_secs, "enabling request timeout layer");
            app = app.layer(TimeoutLayer::with_status_code(
                axum::http::StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(timeout_secs),
            ));
        }

        if self.config.cors_permissive {
            tracing::debug!("enabling permissive CORS layer");
            app = app.layer(CorsLayer::permissive());
        }

        // Attach metrics registry via Extension and enable HTTP metrics middleware when configured
        if let Some(registry) = &self.config.metrics_registry {
            app = app.layer(Extension(std::sync::Arc::clone(registry)));
            // Use route_layer so MatchedPath is set before middleware runs
            app = app.route_layer(middleware::HttpMetricsLayer::new(std::sync::Arc::clone(
                registry,
            )));
            tracing::debug!("http metrics layer attached to router");
            // Attach HTTP endpoint-specific metrics (e.g., /healthz)
            let healthz_metrics = std::sync::Arc::new(healthz::HealthzMetrics::new(
                std::sync::Arc::clone(registry),
            ));
            app = app.layer(Extension(healthz_metrics));
        }

        // Create listener
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ServiceError::Other(format!("listener bind failed: {}", e)))?;
        tracing::info!(addr=%addr, "listener created and bound");
        // If ephemeral port (0) requested, capture the actual bound port and update config for clarity.
        if self.config.service.port == 0 {
            if let Ok(bound) = listener.local_addr() {
                tracing::info!("HTTP server listening on {}", bound);
            } else {
                tracing::info!("HTTP server listening on {} (ephemeral)", addr);
            }
        } else if let Ok(bound) = listener.local_addr() {
            tracing::info!("HTTP server listening on {}", bound);
        } else {
            tracing::info!("HTTP server listening on {}", addr);
        }

        // Mark ready and start serving; export readiness gauge if registry present
        let mut ready_gauge: Option<Arc<dyn crate::util::metrics::MetricGauge + Send + Sync>> =
            None;
        if let Some(registry) = &self.config.metrics_registry {
            let g = registry.gauge("http_ready", "HTTP server readiness flag", &[]);
            g.set(1.0);
            ready_gauge = Some(g.into());
        }
        self.ready.store(true, Ordering::Relaxed);
        tracing::info!(ready = true, "server marked ready");
        tracing::debug!("ready flag set; entering serve loop");

        // Serve and measure graceful shutdown timing
        let serve_start = Instant::now();
        tracing::info!("starting HTTP serve loop");
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| ServiceError::Other(format!("http serve failed: {}", e)))?;
        tracing::info!(duration_ms=%serve_start.elapsed().as_millis(), "http serve completed/shutdown signal received");

        // Invoke graceful cleanup hooks after server stops (Ctrl+C or error)
        {
            let service = self.service.lock().await;
            tracing::info!("invoking service.stop() as part of graceful shutdown");
            if let Err(err) = service.stop().await {
                tracing::warn!(error = ?err, "service stop hook failed");
            }
            tracing::info!("invoking service.shutdown() as part of graceful shutdown");
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
    /// Optional label provider to derive metrics labels without depending on config.
    pub label_provider:
        Option<std::sync::Arc<dyn crate::services::robotorq_service::MetricsLabelProvider>>,
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
            .field(
                "metrics_registry",
                &self.metrics_registry.as_ref().map(|_| "<registry>"),
            )
            .field(
                "label_provider",
                &self.label_provider.as_ref().map(|_| "<provider>"),
            )
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
            label_provider: None,
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
            label_provider: None,
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

    /// Provide a metrics label provider to decouple label derivation.
    pub fn with_label_provider(
        mut self,
        provider: std::sync::Arc<dyn crate::services::robotorq_service::MetricsLabelProvider>,
    ) -> Self {
        self.label_provider = Some(provider);
        self
    }
}

/// Builder for configuring and constructing an `HttpServer` with fluent, staged options.
///
/// This encapsulates incremental configuration (address/port, timeouts, body limits,
/// CORS, metrics registry, static label provider) before producing the final
/// `HttpServer`. It reduces per‑service boilerplate when multiple optional
/// observability components are in play.
///
/// # Example
/// ```rust,no_run
/// use std::sync::Arc; use tokio::sync::Mutex;
/// use commons::services::robotorq_service::{HttpServerBuilder, RoboTorqService};
/// struct Svc;
/// impl commons::services::robotorq_service::ServiceLifecycle for Svc {}
/// impl commons::services::robotorq_service::HealthContributor for Svc { fn health_status(&self) -> String { "OK".to_string() } }
/// impl commons::services::robotorq_service::MetricsContributor for Svc {}
/// impl RoboTorqService for Svc {}
/// let svc = Arc::new(Mutex::new(Svc));
/// let builder = HttpServerBuilder::new(svc)
///     .with_port(0)
///     .with_manual_registry("svc_name", "http", "0.1.0")
///     .with_static_labels("svc_name", "http", "0.1.0", "core");
/// // Build without starting:
/// let server = builder.build();
/// // Or autoload + start:
/// // tokio::spawn(async move { builder.build_and_start_autoload().await.unwrap(); });
/// ```
pub struct HttpServerBuilder<S: RoboTorqService + Send + Sync + 'static> {
    service: Arc<Mutex<S>>,
    config: HttpServerConfig,
}

impl<S: RoboTorqService + Send + Sync + 'static> HttpServerBuilder<S> {
    /// Create a new builder with default local configuration (localhost, ephemeral port).
    pub fn new(service: Arc<Mutex<S>>) -> Self {
        Self {
            service,
            config: HttpServerConfig::local_defaults(0),
        }
    }

    /// Override port while keeping existing address.
    pub fn with_port(mut self, port: u16) -> Self {
        self.config.service.port = port;
        tracing::debug!(port = port, "HttpServerBuilder: with_port set");
        self
    }
    /// Override bind address.
    pub fn with_address<Saddr: Into<String>>(mut self, address: Saddr) -> Self {
        self.config.service.address = address.into();
        tracing::debug!(address = %self.config.service.address, "HttpServerBuilder: with_address set");
        self
    }
    /// Set request timeout seconds.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.config.timeout_seconds = Some(secs);
        tracing::debug!(timeout_secs = secs, "HttpServerBuilder: with_timeout set");
        self
    }
    /// Remove request timeout.
    pub fn without_timeout(mut self) -> Self {
        self.config.timeout_seconds = None;
        tracing::debug!("HttpServerBuilder: without_timeout called");
        self
    }
    /// Set maximum body size in bytes.
    pub fn with_body_limit(mut self, bytes: usize) -> Self {
        self.config.max_body_size_bytes = Some(bytes);
        tracing::debug!(body_limit = bytes, "HttpServerBuilder: with_body_limit set");
        self
    }
    /// Disable body size limit.
    pub fn without_body_limit(mut self) -> Self {
        self.config.max_body_size_bytes = None;
        tracing::debug!("HttpServerBuilder: without_body_limit called");
        self
    }
    /// Configure permissive CORS.
    pub fn with_cors_permissive(mut self, permissive: bool) -> Self {
        self.config.cors_permissive = permissive;
        tracing::debug!(
            cors_permissive = permissive,
            "HttpServerBuilder: with_cors_permissive set"
        );
        self
    }
    /// Attach an existing metrics registry.
    pub fn with_registry(
        mut self,
        registry: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry>,
    ) -> Self {
        self.config.metrics_registry = Some(registry);
        tracing::debug!("HttpServerBuilder: explicit registry attached");
        self
    }
    /// Construct and attach a Prometheus registry with base labels.
    pub fn with_manual_registry(mut self, service: &str, component: &str, version: &str) -> Self {
        let reg: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> = std::sync::Arc::new(
            crate::util::metrics::PrometheusRegistry::new(service, component, version),
        );
        self.config.metrics_registry = Some(reg);
        tracing::debug!(service = %service, component = %component, version = %version, "HttpServerBuilder: with_manual_registry created and attached");
        self
    }
    /// Provide a static label set independent of `RoboTorqConfig`.
    pub fn with_static_labels(
        mut self,
        service: &str,
        component: &str,
        version: &str,
        subject: &str,
    ) -> Self {
        struct StaticLabelSetProvider(LabelSet);
        impl MetricsLabelProvider for StaticLabelSetProvider {
            fn labels(&self) -> LabelSet {
                self.0.clone()
            }
        }
        let provider = StaticLabelSetProvider(LabelSet {
            service: service.to_string(),
            component: component.to_string(),
            version: version.to_string(),
            subject: subject.to_string(),
        });
        self.config.label_provider = Some(std::sync::Arc::new(provider));
        tracing::debug!(service = %service, component = %component, version = %version, subject = %subject, "HttpServerBuilder: with_static_labels applied");
        self
    }
    /// Finalize builder returning an `HttpServer` (not started).
    pub fn build(self) -> HttpServer<S> {
        HttpServer::new(self.service, self.config)
    }
    /// Build and start with provided config.
    pub async fn build_and_start(self, cfg: &RoboTorqConfig) -> Result<(), ServiceError> {
        self.build().start(cfg).await
    }
    /// Build and start using autoloaded config.
    pub async fn build_and_start_autoload(self) -> Result<(), ServiceError> {
        self.build().start_autoload().await
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
        fn health_check(&self) -> Result<String, ServiceError> {
            Ok("OK".to_string())
        }

        fn export_metrics(&self) -> String {
            self.handler.export_text()
        }

        async fn initialize(&mut self, _config: &RoboTorqConfig) -> Result<(), ServiceError> {
            Ok(())
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

    impl ServiceLifecycle for TestService {}
    impl HealthContributor for TestService {
        fn health_status(&self) -> String {
            self.health_check().unwrap_or_default()
        }
    }
    impl MetricsContributor for TestService {
        fn set_metrics_context(
            &mut self,
            _ctx: Option<std::sync::Arc<service_metrics_context::ServiceMetricsContext>>,
        ) {
        }
        fn export_metrics(&self) -> String {
            <TestService as RoboTorqService>::export_metrics(self)
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

    #[test]
    fn builder_with_variants_sets_config() {
        use crate::util::metrics::PrometheusRegistry;

        let handler = MetricsHandler::new();
        let svc = Arc::new(Mutex::new(TestService { handler }));
        let reg: std::sync::Arc<dyn crate::util::metrics::MetricsRegistry> =
            std::sync::Arc::new(PrometheusRegistry::new("svc", "http", "vtest"));

        let builder = HttpServerBuilder::new(Arc::clone(&svc))
            .with_port(12345)
            .with_address("127.0.0.2")
            .with_timeout(5)
            .with_body_limit(4096)
            .with_cors_permissive(false)
            .with_registry(std::sync::Arc::clone(&reg))
            .with_static_labels("svc", "http", "vtest", "core");

        let server = builder.build();
        // Check simple fields
        assert_eq!(server.config.service.port, 12345);
        assert_eq!(server.config.service.address, "127.0.0.2");
        assert_eq!(server.config.timeout_seconds, Some(5));
        assert_eq!(server.config.max_body_size_bytes, Some(4096));
        assert!(!server.config.cors_permissive);
        // Metrics registry present and label provider present
        assert!(server.config.metrics_registry.is_some());
        assert!(server.config.label_provider.is_some());
    }
}
