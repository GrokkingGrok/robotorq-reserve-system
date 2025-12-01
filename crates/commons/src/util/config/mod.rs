//! Configuration Management for RoboTorq Reserve System
//!
//! This module provides a comprehensive configuration system supporting multiple formats
//! (TOML and JSON) with schema versioning for the RoboTorq Reserve System.
//!
//! # Configuration Architecture
//!
//! The system supports hierarchical configuration with:
//! - **Mode Configuration**: Production vs Simulation operation modes
//! - **Simulation Parameters**: Timing, performance, and robot gateway simulation settings
//! - **Port Configuration**: Network port assignments for services and monitoring
//! - **HTTP Configuration**: Web server settings with middleware and security
//! - **NATS Configuration**: Message bus connection and JetStream settings
//! - **Persistence Configuration**: Database connection and storage settings
//! - **Observability Configuration**: Metrics, tracing, and monitoring settings
//! - **Security Configuration**: TLS, authentication, and authorization settings
//! - **Crypto Configuration**: Cryptographic signing, key management, and certificate settings
//! - **Economic Configuration**: Economic invariants, reserve ratios, and monetary policy settings
//!
//! # Configuration Sources
//!
//! Configurations can be loaded from:
//! - TOML files (recommended for human-editable configs)
//! - JSON files (for programmatic generation)
//! - Environment variables (via serde rename attributes)
//!
//! # Schema Versioning
//!
//! The configuration system uses schema versioning to ensure compatibility:
//! - Schema version is validated against `ROBOTORQ_CONFIG_SCHEMA_VERSION`
//! - Version mismatches prevent configuration loading
//! - Future versions can introduce migrations if needed
//!
//! # Usage Examples
//!
//! ```rust,no_run
//! use commons::util::config::RoboTorqConfig;
//!
//! // Load from TOML file
//! let config = RoboTorqConfig::load_from_file("robotorq.toml".as_ref()).unwrap();
//!
//! // Access configuration sections
//! match config.mode {
//!     commons::util::config::Mode::Production => {
//!         // Production-specific setup
//!     }
//!     commons::util::config::Mode::Simulation => {
//!         // Simulation-specific setup
//!     }
//! }
//! ```

pub mod mode;
pub mod ports;
pub mod simulation;
// Nested service-specific config modules live under `services/`
/// Service-specific configuration modules.
///
/// This namespace groups configuration for transport adapters and
/// messaging backbones used by services (e.g., HTTP and NATS).
pub mod services {
    pub mod http;
    pub mod nats;
}
// Persistence config lives under `persistance/` (intentional spelling per repo layout)
pub mod crypto;
pub mod economic;
pub mod observability;
pub mod persistance;
pub mod security;

// Re-export common config types for ergonomic imports
pub use crypto::CryptoConfig;
pub use economic::EconomicConfig;
pub use mode::Mode;
pub use observability::ObservabilityConfig;
pub use persistance::PersistenceConfig;
pub use ports::PortsConfig;
pub use ports::load_ports_config_from_default;
pub use security::SecurityConfig;
pub use services::http::HttpConfig;
pub use services::nats::NatsConfig;
pub use simulation::Simulation;
pub use simulation::sim_sleep;

use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

/// Main configuration structure for the RoboTorq Reserve System.
///
/// This struct encapsulates all configuration parameters needed to operate
/// the RoboTorq system, including operational mode, simulation settings,
/// network configuration, and service-specific settings.
///
/// # Configuration Sections
///
/// - `schema_version`: Version of the configuration schema for compatibility
/// - `mode`: Operational mode (Production or Simulation)
/// - `simulation`: Simulation-specific parameters (only used in Simulation mode)
/// - `ports`: Network port assignments for all services
/// - `http`: HTTP server configuration (middleware, CORS, timeouts)
/// - `nats`: NATS message bus configuration (servers, JetStream)
/// - `persistence`: Database and storage configuration
/// - `observability`: Metrics, tracing, and monitoring settings
/// - `security`: TLS, authentication, and authorization settings
/// - `crypto`: Cryptographic signing, key management, and certificate settings
/// - `economic`: Economic invariants, reserve ratios, and monetary policy settings
///
/// # Validation
///
/// Configuration is validated during deserialization. Invalid values will
/// cause loading to fail with a descriptive error message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqConfig {
    /// Configuration schema version for compatibility checking.
    ///
    /// This version must match `ROBOTORQ_CONFIG_SCHEMA_VERSION` from the schema module.
    /// Schema versioning ensures configuration compatibility across system versions.
    ///
    /// Defaults to the current schema version if not specified.
    #[serde(default = "default_robotorq_config_schema_version")]
    pub schema_version: u32,

    /// Operational mode of the RoboTorq system.
    ///
    /// Determines whether the system runs in production or simulation mode.
    /// Production mode connects to real robots and manages actual economic transactions.
    /// Simulation mode uses virtual robots and simulated economic activity.
    pub mode: Mode,

    /// Simulation configuration parameters.
    ///
    /// These settings are only used when `mode` is set to `Simulation`.
    /// Contains timing controls, performance parameters, and robot gateway simulation settings.
    pub simulation: Simulation,

    /// Network port configuration for all services.
    ///
    /// Defines the ports on which various system services will listen,
    /// including the robot gateway, metrics endpoint, and Grafana dashboard.
    pub ports: ports::PortsConfig,

    /// HTTP server configuration.
    ///
    /// Configures the HTTP server including middleware settings, CORS policy,
    /// request timeouts, and body size limits.
    #[serde(default)]
    pub http: HttpConfig,

    /// NATS message bus configuration.
    ///
    /// Defines connection settings for the NATS server, JetStream configuration,
    /// and subject naming conventions for inter-service communication.
    #[serde(default)]
    pub nats: NatsConfig,

    /// Persistence layer configuration.
    ///
    /// Configures database connections, connection pooling, and storage settings
    /// for the unified persistence strategy across all services.
    #[serde(default)]
    pub persistence: PersistenceConfig,

    /// Observability configuration.
    ///
    /// Settings for metrics collection, tracing, and monitoring integration
    /// including Prometheus endpoints and Grafana dashboard connections.
    #[serde(default)]
    pub observability: ObservabilityConfig,

    /// Security configuration.
    ///
    /// TLS settings, authentication mechanisms, and authorization policies
    /// for securing service communication and external access.
    #[serde(default)]
    pub security: SecurityConfig,

    /// Cryptographic configuration.
    ///
    /// Settings for digital signatures, key management, and certificate handling
    /// including post-quantum algorithms like Falcon and SPHINCS.
    #[serde(default)]
    pub crypto: CryptoConfig,

    /// Economic configuration.
    ///
    /// Settings for economic invariants, reserve ratios, monetary policy,
    /// and token economics including demurrage rates and UBD parameters.
    #[serde(default)]
    pub economic: EconomicConfig,
}

/// Returns the default schema version for new configurations.
///
/// This function provides the current schema version as the default value
/// when deserializing configurations that don't specify a schema version.
fn default_robotorq_config_schema_version() -> u32 {
    ROBOTORQ_CONFIG_SCHEMA_VERSION
}

impl RoboTorqConfig {
    /// Load configuration from a file supporting both TOML and JSON formats.
    ///
    /// This method automatically detects the file format based on the file extension
    /// and parses the configuration accordingly. Both TOML (.toml) and JSON (.json)
    /// formats are supported for maximum flexibility.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// Returns `Ok(RoboTorqConfig)` if the file is successfully parsed and validated,
    /// or `Err(String)` containing a descriptive error message if loading fails.
    ///
    /// # Errors
    ///
    /// This function can fail due to:
    /// - File not found or permission denied
    /// - Invalid file format or syntax errors
    /// - Schema version mismatches
    /// - Missing required configuration fields
    /// - Invalid configuration values
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use commons::util::config::RoboTorqConfig;
    /// // Load from TOML file
    /// let config = RoboTorqConfig::load_from_file("robotorq.toml".as_ref())?;
    /// # Ok::<(), String>(())
    /// ```
    ///
    /// ```rust,no_run
    /// # use commons::util::config::RoboTorqConfig;
    /// // Load from JSON file
    /// let config = RoboTorqConfig::load_from_file("config.json".as_ref())?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).map_err(|e| e.to_string())
        } else {
            toml::from_str(&content).map_err(|e| e.to_string())
        }
    }
}

/// Load RoboTorq configuration with fallback logic.
///
/// This function provides a unified way to load configuration across all services.
/// It tries multiple sources in order:
/// 1. Command line argument (if provided)
/// 2. ROBOTORQ_CONFIG environment variable
/// 3. Default config file paths
/// 4. Built-in defaults
///
/// # Arguments
///
/// * `config_path` - Optional path to config file. If None, uses environment/default paths.
///
/// # Returns
///
/// Returns the loaded configuration or an error message.
///
/// # Examples
///
/// ```rust,no_run
/// use commons::util::config::load_robotorq_config;
///
/// // Load from default locations
/// let config = load_robotorq_config(None).map_err(ConfigError::Invalid)?;
///  info!(
///     schema_version = config.schema_version,
///     mode = ?config.mode,
///     http_address = %config.http.address,
///     http_port = config.http.port,
///     "loaded RoboTorq configuration"
/// );
///     // Build the HTTP server config from the loaded settings, honoring the sandbox override.
/// let http_config = build_http_server_config(&config.http, Some(port));
///
/// let service = Arc::new(SandboxService::new());
/// info!(
///     port = http_config.service.port,
///     "starting the RoboTorq sandbox"
/// );
/// ```
///
/// # Errors
///
/// This function currently does not return any errors but may in the future
/// when configuration validation or parsing fails.
pub fn load_robotorq_config(
    config_path: Option<&std::path::Path>,
) -> Result<RoboTorqConfig, String> {
    // Try explicit path first
    if let Some(path) = config_path {
        match RoboTorqConfig::load_from_file(path) {
            Ok(config) => return Ok(config),
            Err(e) => tracing::warn!("Failed to load config from {}: {}", path.display(), e),
        }
    }

    // Try environment variable
    if let Ok(env_path) = std::env::var("ROBOTORQ_CONFIG") {
        let path = std::path::Path::new(&env_path);
        match RoboTorqConfig::load_from_file(path) {
            Ok(config) => return Ok(config),
            Err(e) => tracing::warn!(
                "Failed to load config from ROBOTORQ_CONFIG={}: {}",
                env_path,
                e
            ),
        }
    }

    // Try default locations
    let default_paths = ["robotorq.toml", "config/robotorq.toml", "etc/robotorq.toml"];

    for path_str in &default_paths {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            match RoboTorqConfig::load_from_file(path) {
                Ok(config) => {
                    tracing::info!("Loaded config from {}", path.display());
                    return Ok(config);
                }
                Err(e) => tracing::warn!("Failed to load config from {}: {}", path.display(), e),
            }
        }
    }

    // Fall back to defaults
    tracing::warn!("No config file found, using defaults");
    Ok(RoboTorqConfig {
        schema_version: default_robotorq_config_schema_version(),
        mode: Mode::Production, // Safe default
        simulation: Simulation::default(),
        ports: load_ports_config_from_default(),
        http: HttpConfig::default(),
        nats: NatsConfig::default(),
        persistence: PersistenceConfig::default(),
        observability: ObservabilityConfig::default(),
        security: SecurityConfig::default(),
        crypto: CryptoConfig::default(),
        economic: EconomicConfig::default(),
    })
}
