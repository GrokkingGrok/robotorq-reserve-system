//! Observability configuration for `RoboTorq` services.
//!
//! This module defines configuration options for metrics collection,
//! tracing, logging, and monitoring integration.

use serde::{Deserialize, Serialize};

/// Observability configuration.
///
/// Configures metrics collection, tracing, and monitoring for the `RoboTorq` system.
/// Strong observability is critical for operating distributed systems reliably.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::ObservabilityConfig;
///
/// // Production observability configuration
/// let prod_config = ObservabilityConfig {
///     service_name: "robot-gateway".to_string(),
///     service_instance: "gateway-01".to_string(),
///     service_version: "1.2.3".to_string(),
///     ..Default::default()
/// };
///
/// // Development observability configuration
/// let dev_config = ObservabilityConfig {
///     service_name: "robot-gateway".to_string(),
///     service_instance: "dev-local".to_string(),
///     service_version: "dev".to_string(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Service identification for metrics and traces.
    ///
    /// Used as a label in metrics and as the service name in traces.
    /// Should be unique per service type (e.g., "robot-gateway", "refinery").
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Service instance identifier.
    ///
    /// Distinguishes between multiple instances of the same service.
    /// Useful for horizontal scaling and debugging.
    #[serde(default = "default_service_instance")]
    pub service_instance: String,

    /// Service version.
    ///
    /// Version string included in metrics and traces for tracking deployments.
    #[serde(default = "default_service_version")]
    pub service_version: String,

    /// Metrics collection configuration.
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// Tracing configuration.
    #[serde(default)]
    pub tracing: TracingConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Health monitoring configuration.
    #[serde(default)]
    pub health: HealthConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            service_instance: default_service_instance(),
            service_version: default_service_version(),
            metrics: MetricsConfig::default(),
            tracing: TracingConfig::default(),
            logging: LoggingConfig::default(),
            health: HealthConfig::default(),
        }
    }
}

/// Metrics collection configuration.
///
/// Configures Prometheus metrics collection and export.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::{MetricsConfig, MetricsRegistryType};
/// use std::collections::HashMap;
///
/// // Production metrics configuration
/// let mut common_labels = HashMap::new();
/// common_labels.insert("environment".to_string(), "production".to_string());
/// common_labels.insert("region".to_string(), "us-west-2".to_string());
///
/// let prod_metrics = MetricsConfig {
///     enabled: true,
///     registry: MetricsRegistryType::Prometheus,
///     common_labels,
///     path: "/metrics".to_string(),
///     collection_interval_seconds: 30,
/// };
///
/// // Minimal metrics configuration
/// let minimal_metrics = MetricsConfig {
///     enabled: false,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics collection is enabled.
    ///
    /// When disabled, services will not collect or expose metrics.
    /// Useful for testing or when metrics infrastructure is unavailable.
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,

    /// Metrics registry type.
    ///
    /// Determines how metrics are stored and exposed.
    #[serde(default)]
    pub registry: MetricsRegistryType,

    /// Common labels applied to all metrics.
    ///
    /// Labels that are added to every metric emitted by the service.
    /// Common examples: environment, region, cluster.
    #[serde(default = "default_common_labels")]
    pub common_labels: std::collections::HashMap<String, String>,

    /// Metrics endpoint path.
    ///
    /// HTTP path where Prometheus metrics are exposed.
    /// Must match the HTTP server metrics endpoint configuration.
    #[serde(default = "default_metrics_path")]
    pub path: String,

    /// Metrics collection interval in seconds.
    ///
    /// How often to update gauge metrics that reflect current state.
    /// Does not affect counter/histogram collection which is event-driven.
    #[serde(default = "default_collection_interval_seconds")]
    pub collection_interval_seconds: u64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            registry: MetricsRegistryType::default(),
            common_labels: default_common_labels(),
            path: default_metrics_path(),
            collection_interval_seconds: default_collection_interval_seconds(),
        }
    }
}

/// Metrics registry types.
///
/// Determines how metrics are stored and exposed.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::MetricsRegistryType;
///
/// // Standard Prometheus metrics (recommended)
/// let prometheus = MetricsRegistryType::Prometheus;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MetricsRegistryType {
    /// Prometheus metrics registry.
    ///
    /// Standard Prometheus text format metrics for scraping.
    #[default]
    Prometheus,
}

/// Tracing configuration.
///
/// Configures distributed tracing for request tracking across services.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::{TracingConfig, TracingBackend};
///
/// // Production tracing with Jaeger
/// let prod_tracing = TracingConfig {
///     enabled: true,
///     backend: TracingBackend::Jaeger,
///     sampling_rate: 0.1, // 10% sampling
///     service_name_override: None,
/// };
///
/// // Development tracing to console
/// let dev_tracing = TracingConfig {
///     enabled: true,
///     backend: TracingBackend::Console,
///     sampling_rate: 1.0, // 100% sampling
///     service_name_override: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Whether tracing is enabled.
    ///
    /// When disabled, tracing calls become no-ops for performance.
    #[serde(default = "default_tracing_enabled")]
    pub enabled: bool,

    /// Tracing backend.
    ///
    /// Determines where traces are sent and how they're formatted.
    #[serde(default)]
    pub backend: TracingBackend,

    /// Sampling rate (0.0 to 1.0).
    ///
    /// Fraction of traces to collect. 1.0 = all traces, 0.1 = 10% of traces.
    /// Lower rates reduce overhead but may miss important traces.
    #[serde(default = "default_sampling_rate")]
    pub sampling_rate: f64,

    /// Service name override.
    ///
    /// If set, overrides the global `service_name` for tracing.
    /// Useful when multiple services run in the same process.
    #[serde(default)]
    pub service_name_override: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: default_tracing_enabled(),
            backend: TracingBackend::default(),
            sampling_rate: default_sampling_rate(),
            service_name_override: None,
        }
    }
}

/// Tracing backends.
///
/// Determines where traces are sent and how they're formatted.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::TracingBackend;
///
/// // OpenTelemetry with OTLP (flexible, recommended for production)
/// let otel_otlp = TracingBackend::OtelOtlp;
///
/// // Direct Jaeger export
/// let jaeger = TracingBackend::Jaeger;
///
/// // Console output for development
/// let console = TracingBackend::Console;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TracingBackend {
    /// OpenTelemetry with OTLP exporter.
    ///
    /// Industry standard tracing with flexible backends (Jaeger, Zipkin, etc.).
    OtelOtlp,

    /// Jaeger native protocol.
    ///
    /// Direct export to Jaeger for distributed tracing.
    Jaeger,

    /// Console logging (development only).
    ///
    /// Prints traces to stdout/stderr. Not suitable for production.
    #[default]
    Console,
}

/// Logging configuration.
///
/// Configures structured logging for services.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::{LoggingConfig, LogLevel, LogFormat};
/// use std::collections::HashMap;
///
/// // Production JSON logging
/// let mut prod_fields = HashMap::new();
/// prod_fields.insert("environment".to_string(), "production".to_string());
///
/// let prod_logging = LoggingConfig {
///     level: LogLevel::Info,
///     format: LogFormat::Json,
///     timestamps: true,
///     source_location: false,
///     fields: prod_fields,
/// };
///
/// // Development pretty logging
/// let dev_logging = LoggingConfig {
///     level: LogLevel::Debug,
///     format: LogFormat::Pretty,
///     timestamps: true,
///     source_location: true,
///     fields: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level.
    ///
    /// Minimum log level to output. Messages below this level are filtered.
    #[serde(default)]
    pub level: LogLevel,

    /// Log format.
    ///
    /// How log messages are formatted for output.
    #[serde(default)]
    pub format: LogFormat,

    /// Whether to include timestamps in logs.
    #[serde(default = "default_log_timestamps")]
    pub timestamps: bool,

    /// Whether to include source location in logs.
    #[serde(default = "default_log_source_location")]
    pub source_location: bool,

    /// Additional fields to include in all log messages.
    #[serde(default = "default_log_fields")]
    pub fields: std::collections::HashMap<String, String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            format: LogFormat::default(),
            timestamps: default_log_timestamps(),
            source_location: default_log_source_location(),
            fields: default_log_fields(),
        }
    }
}

/// Log levels.
///
/// Minimum log level to output. Messages below this level are filtered.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::LogLevel;
///
/// // Only show errors
/// let error_only = LogLevel::Error;
///
/// // Show warnings and above
/// let warn_plus = LogLevel::Warn;
///
/// // Show info messages and above (recommended for production)
/// let info_plus = LogLevel::Info;
///
/// // Show debug messages and above (development)
/// let debug_plus = LogLevel::Debug;
///
/// // Show all messages including traces (verbose debugging)
/// let trace_all = LogLevel::Trace;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Only error messages
    Error,
    /// Warnings and above
    Warn,
    /// Info messages and above
    #[default]
    Info,
    /// Debug messages and above
    Debug,
    /// All messages including traces
    Trace,
}

/// Log formats.
///
/// How log messages are formatted for output.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::LogFormat;
///
/// // Human-readable format with colors (development)
/// let pretty = LogFormat::Pretty;
///
/// // Structured JSON format (production, log aggregation)
/// let json = LogFormat::Json;
///
/// // Compact single-line format (system logs)
/// let compact = LogFormat::Compact;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    /// Human-readable format
    #[default]
    Pretty,
    /// JSON structured format
    Json,
    /// Compact single-line format
    Compact,
}

/// Health monitoring configuration.
///
/// Configures health checks and readiness probes.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::observability::HealthConfig;
///
/// // Production health checks
/// let prod_health = HealthConfig {
///     check_interval_seconds: 30,
///     check_timeout_seconds: 5,
///     failure_threshold: 3,
///     initial_healthy: false,
/// };
///
/// // Fast health checks for development
/// let dev_health = HealthConfig {
///     check_interval_seconds: 10,
///     check_timeout_seconds: 2,
///     failure_threshold: 2,
///     initial_healthy: false,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Health check interval in seconds.
    ///
    /// How often to run internal health checks.
    #[serde(default = "default_health_check_interval_seconds")]
    pub check_interval_seconds: u64,

    /// Health check timeout in seconds.
    ///
    /// Maximum time allowed for health checks to complete.
    #[serde(default = "default_health_check_timeout_seconds")]
    pub check_timeout_seconds: u64,

    /// Number of consecutive failures before marking unhealthy.
    ///
    /// Service remains healthy until this many consecutive health checks fail.
    #[serde(default = "default_health_failure_threshold")]
    pub failure_threshold: u32,

    /// Initial health state.
    ///
    /// Whether the service starts as healthy or unhealthy.
    /// Should generally be false until initialization completes.
    #[serde(default = "default_health_initial_state")]
    pub initial_healthy: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_health_check_interval_seconds(),
            check_timeout_seconds: default_health_check_timeout_seconds(),
            failure_threshold: default_health_failure_threshold(),
            initial_healthy: default_health_initial_state(),
        }
    }
}

// Default value functions

/// Returns the default service name.
///
/// Returns `"robotorq-service"` as a generic service identifier.
/// Should be overridden with specific service names in configuration.
fn default_service_name() -> String {
    "robotorq-service".to_string()
}

/// Returns the default service instance identifier.
///
/// Returns `"default"` for single-instance deployments.
/// Should be overridden with unique identifiers in multi-instance setups.
fn default_service_instance() -> String {
    "default".to_string()
}

/// Returns the default service version.
///
/// Returns the Cargo package version from build environment.
/// Automatically tracks deployed version for observability.
fn default_service_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Returns the default metrics enabled state.
///
/// Returns `true` to enable metrics collection by default.
/// Essential for monitoring and alerting in production.
fn default_metrics_enabled() -> bool {
    true
}

/// Returns the default common metric labels.
///
/// Returns a map with `"environment": "development"` for basic labeling.
/// Should be customized for production deployments.
fn default_common_labels() -> std::collections::HashMap<String, String> {
    let mut labels = std::collections::HashMap::new();
    labels.insert("environment".to_string(), "development".to_string());
    labels
}

/// Returns the default metrics endpoint path.
///
/// Returns `"/metrics"` to match standard Prometheus scraping expectations.
/// Must align with HTTP server endpoint configuration.
fn default_metrics_path() -> String {
    "/metrics".to_string()
}

/// Returns the default metrics collection interval.
///
/// Returns `60` seconds for periodic gauge updates.
/// Balances monitoring freshness with performance overhead.
fn default_collection_interval_seconds() -> u64 {
    60
}

/// Returns the default tracing enabled state.
///
/// Returns `true` to enable tracing by default.
/// Critical for debugging distributed systems.
fn default_tracing_enabled() -> bool {
    true
}

/// Returns the default tracing sampling rate.
///
/// Returns `1.0` (100% sampling) for complete trace capture.
/// Should be reduced in high-traffic production systems.
fn default_sampling_rate() -> f64 {
    1.0
}

/// Returns the default log timestamp inclusion.
///
/// Returns `true` to include timestamps in all log messages.
/// Essential for log aggregation and debugging.
fn default_log_timestamps() -> bool {
    true
}

/// Returns the default source location inclusion in logs.
///
/// Returns `false` to exclude source locations by default.
/// Can be enabled for detailed debugging but adds verbosity.
fn default_log_source_location() -> bool {
    false
}

/// Returns the default additional log fields.
///
/// Returns an empty map for custom log fields.
/// Can be populated with service-specific metadata.
fn default_log_fields() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

/// Returns the default health check interval.
///
/// Returns `30` seconds between health checks.
/// Balances monitoring frequency with system overhead.
fn default_health_check_interval_seconds() -> u64 {
    30
}

/// Returns the default health check timeout.
///
/// Returns `5` seconds as the maximum time for health checks.
/// Prevents health checks from blocking service operation.
fn default_health_check_timeout_seconds() -> u64 {
    5
}

/// Returns the default health check failure threshold.
///
/// Returns `3` consecutive failures before marking unhealthy.
/// Provides resilience against transient failures.
fn default_health_failure_threshold() -> u32 {
    3
}

/// Returns the default initial health state.
///
/// Returns `false` to start unhealthy until initialization completes.
/// Ensures services aren't considered healthy before they're ready.
fn default_health_initial_state() -> bool {
    false
}
