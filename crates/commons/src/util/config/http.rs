//! HTTP server configuration for RoboTorq services.
//!
//! This module defines configuration options for HTTP servers including
//! middleware settings, CORS policies, timeouts, and security options.

use serde::{Deserialize, Serialize};

/// HTTP server configuration.
///
/// Configures the HTTP server behavior including middleware, security settings,
/// and performance parameters. These settings control how services expose
/// their HTTP interfaces for monitoring, health checks, and external APIs.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::http::{HttpConfig, CorsConfig};
///
/// // Development configuration with permissive CORS
/// let dev_config = HttpConfig {
///     enabled: true,
///     address: "127.0.0.1".to_string(),
///     port: 8080,
///     request_timeout_seconds: Some(30),
///     max_body_size_bytes: Some(1024 * 1024),
///     cors: CorsConfig {
///         enabled: true,
///         allowed_origins: vec!["*".to_string()],
///         ..Default::default()
///     },
///     ..Default::default()
/// };
///
/// // Production configuration with restrictive CORS
/// let prod_config = HttpConfig {
///     enabled: true,
///     address: "0.0.0.0".to_string(),
///     port: 80,
///     request_timeout_seconds: Some(15),
///     max_body_size_bytes: Some(10 * 1024 * 1024),
///     cors: CorsConfig {
///         enabled: true,
///         allowed_origins: vec![
///             "https://dashboard.robotorq.com".to_string(),
///             "https://admin.robotorq.com".to_string(),
///         ],
///         allow_credentials: true,
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Whether to enable the HTTP server.
    ///
    /// When disabled, services will not start an HTTP server. This is useful
    /// for services that only communicate via NATS or for testing scenarios.
    #[serde(default = "default_http_enabled")]
    pub enabled: bool,

    /// Network address to bind the HTTP server to.
    ///
    /// Defaults to "127.0.0.1" (localhost only) for security in development.
    /// Use "0.0.0.0" to bind to all interfaces in production.
    #[serde(default = "default_http_address")]
    pub address: String,

    /// Port number for the HTTP server.
    ///
    /// The port on which the HTTP server will listen for connections.
    /// Common values: 8080 (development), 80/443 (production with/without TLS).
    #[serde(default = "default_http_port")]
    pub port: u16,

    /// Request timeout in seconds.
    ///
    /// Maximum time to wait for a complete HTTP request. Requests exceeding
    /// this timeout will be terminated with a 408 Request Timeout response.
    /// Set to None to disable timeouts.
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: Option<u64>,

    /// Maximum request body size in bytes.
    ///
    /// Limits the size of request bodies to prevent abuse. Requests with
    /// bodies larger than this limit will receive a 413 Payload Too Large response.
    /// Set to None for unlimited body size (not recommended for production).
    #[serde(default = "default_max_body_size_bytes")]
    pub max_body_size_bytes: Option<usize>,

    /// CORS policy configuration.
    ///
    /// Controls Cross-Origin Resource Sharing settings. In development,
    /// permissive CORS is often enabled. In production, restrict to specific origins.
    #[serde(default)]
    pub cors: CorsConfig,

    /// Health check endpoint configuration.
    #[serde(default)]
    pub health: EndpointConfig,

    /// Metrics endpoint configuration.
    #[serde(default)]
    pub metrics: EndpointConfig,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: default_http_enabled(),
            address: default_http_address(),
            port: default_http_port(),
            request_timeout_seconds: default_request_timeout_seconds(),
            max_body_size_bytes: default_max_body_size_bytes(),
            cors: CorsConfig::default(),
            health: EndpointConfig::default(),
            metrics: EndpointConfig::default(),
        }
    }
}

/// CORS (Cross-Origin Resource Sharing) configuration.
///
/// Controls which origins, methods, and headers are allowed for cross-origin requests.
/// This is essential for web applications that need to communicate with RoboTorq services.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::http::CorsConfig;
///
/// // Permissive CORS for development
/// let dev_cors = CorsConfig {
///     enabled: true,
///     allowed_origins: vec!["*".to_string()],
///     allowed_methods: vec!["GET".to_string(), "POST".to_string()],
///     allowed_headers: vec!["Content-Type".to_string()],
///     allow_credentials: false,
///     max_age_seconds: 3600,
/// };
///
/// // Restrictive CORS for production
/// let prod_cors = CorsConfig {
///     enabled: true,
///     allowed_origins: vec![
///         "https://dashboard.robotorq.com".to_string(),
///         "https://admin.robotorq.com".to_string(),
///     ],
///     allowed_methods: vec![
///         "GET".to_string(),
///         "POST".to_string(),
///         "PUT".to_string(),
///         "DELETE".to_string(),
///     ],
///     allowed_headers: vec![
///         "Content-Type".to_string(),
///         "Authorization".to_string(),
///         "X-API-Key".to_string(),
///     ],
///     allow_credentials: true,
///     max_age_seconds: 86400,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Whether CORS is enabled.
    ///
    /// When enabled, the server will include CORS headers in responses.
    /// When disabled, CORS headers are omitted (stricter security).
    #[serde(default = "default_cors_enabled")]
    pub enabled: bool,

    /// Allowed origins for CORS requests.
    ///
    /// List of origins that can make cross-origin requests. Use ["*"] for
    /// permissive access (development only) or specify exact domains like
    /// ["https://dashboard.robotorq.com", "https://admin.robotorq.com"].
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods for CORS requests.
    ///
    /// HTTP methods that can be used in cross-origin requests.
    /// Common: ["GET", "POST", "PUT", "DELETE", "OPTIONS"].
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,

    /// Allowed headers for CORS requests.
    ///
    /// HTTP headers that can be included in cross-origin requests.
    /// Common: ["Content-Type", "Authorization", "X-Requested-With"].
    #[serde(default = "default_cors_headers")]
    pub allowed_headers: Vec<String>,

    /// Whether credentials (cookies, authorization headers) can be included.
    ///
    /// When true, allows browsers to include credentials in CORS requests.
    /// Use with caution and only for trusted origins.
    #[serde(default = "default_cors_allow_credentials")]
    pub allow_credentials: bool,

    /// How long CORS preflight results can be cached (in seconds).
    ///
    /// Browsers cache CORS preflight responses to avoid repeated OPTIONS requests.
    /// Higher values reduce server load but delay policy changes.
    #[serde(default = "default_cors_max_age")]
    pub max_age_seconds: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: default_cors_enabled(),
            allowed_origins: default_cors_origins(),
            allowed_methods: default_cors_methods(),
            allowed_headers: default_cors_headers(),
            allow_credentials: default_cors_allow_credentials(),
            max_age_seconds: default_cors_max_age(),
        }
    }
}

/// Endpoint configuration for HTTP routes.
///
/// Defines the path and behavior of specific HTTP endpoints like health checks
/// and metrics endpoints.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::http::EndpointConfig;
///
/// // Standard health check endpoint
/// let health_endpoint = EndpointConfig {
///     path: "/health".to_string(),
///     enabled: true,
/// };
///
/// // Custom metrics endpoint
/// let metrics_endpoint = EndpointConfig {
///     path: "/metrics".to_string(),
///     enabled: true,
/// };
///
/// // Disabled endpoint
/// let disabled_endpoint = EndpointConfig {
///     path: "/debug".to_string(),
///     enabled: false,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// URL path for the endpoint.
    ///
    /// Must start with "/" (e.g., "/health", "/metrics", "/api/v1/status").
    #[serde(default = "default_endpoint_path")]
    pub path: String,

    /// Whether the endpoint is enabled.
    ///
    /// When disabled, the endpoint will return 404 Not Found.
    #[serde(default = "default_endpoint_enabled")]
    pub enabled: bool,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            path: default_endpoint_path(),
            enabled: default_endpoint_enabled(),
        }
    }
}

// Default value functions

/// Returns the default value for HTTP server enabled state.
///
/// Returns `true` to enable HTTP servers by default, allowing services
/// to expose monitoring and health check endpoints.
fn default_http_enabled() -> bool { true }

/// Returns the default network address for HTTP server binding.
///
/// Returns `"127.0.0.1"` (localhost only) for security in development environments.
/// In production, this should be changed to `"0.0.0.0"` to bind to all interfaces.
fn default_http_address() -> String { "127.0.0.1".to_string() }

/// Returns the default port number for HTTP server binding.
///
/// Returns `8080`, a common development port that doesn't conflict with
/// system services running on lower-numbered ports.
fn default_http_port() -> u16 { 8080 }

/// Returns the default request timeout duration.
///
/// Returns `Some(30)` seconds as a reasonable default for most HTTP operations.
/// This prevents requests from hanging indefinitely while allowing time for
/// complex operations like database queries or external API calls.
fn default_request_timeout_seconds() -> Option<u64> { Some(30) }

/// Returns the default maximum request body size.
///
/// Returns `Some(1MB)` to prevent abuse while allowing reasonable payload sizes
/// for configuration updates, bulk operations, and file uploads.
fn default_max_body_size_bytes() -> Option<usize> { Some(1024 * 1024) } // 1MB

/// Returns the default CORS enabled state.
///
/// Returns `true` to enable CORS by default, supporting web applications
/// that need to communicate with RoboTorq services.
fn default_cors_enabled() -> bool { true }

/// Returns the default allowed CORS origins.
///
/// Returns `["*"]` for permissive access in development. In production,
/// this should be restricted to specific trusted domains.
fn default_cors_origins() -> Vec<String> { vec!["*".to_string()] }

/// Returns the default allowed CORS HTTP methods.
///
/// Returns common HTTP methods: `["GET", "POST", "PUT", "DELETE", "OPTIONS"]`.
/// These cover typical REST API operations and CORS preflight requests.
fn default_cors_methods() -> Vec<String> {
    vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()]
}

/// Returns the default allowed CORS headers.
///
/// Returns common headers: `["Content-Type", "Authorization", "X-Requested-With"]`.
/// These support JSON APIs, authentication, and AJAX requests.
fn default_cors_headers() -> Vec<String> {
    vec!["Content-Type".to_string(), "Authorization".to_string(), "X-Requested-With".to_string()]
}

/// Returns the default CORS credentials policy.
///
/// Returns `false` to disable credentials by default for security.
/// When enabled, only specific trusted origins should be allowed.
fn default_cors_allow_credentials() -> bool { false }

/// Returns the default CORS preflight cache duration.
///
/// Returns `86400` seconds (24 hours) to reduce preflight request frequency
/// while allowing reasonable cache invalidation for policy changes.
fn default_cors_max_age() -> u64 { 86400 } // 24 hours

/// Returns the default endpoint path.
///
/// Returns `"/health"` as the standard path for health check endpoints.
/// This follows common conventions for service health monitoring.
fn default_endpoint_path() -> String { "/health".to_string() }

/// Returns the default endpoint enabled state.
///
/// Returns `true` to enable endpoints by default, ensuring monitoring
/// and health check capabilities are available.
fn default_endpoint_enabled() -> bool { true }