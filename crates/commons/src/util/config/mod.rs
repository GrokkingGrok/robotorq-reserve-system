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
pub mod simulation;
pub mod ports;

// Re-export common config types for ergonomic imports
pub use mode::Mode;
pub use simulation::Simulation;
pub use ports::PortsConfig;

use serde::{Deserialize, Serialize};
use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;

/// Main configuration structure for the RoboTorq Reserve System.
///
/// This struct encapsulates all configuration parameters needed to operate
/// the RoboTorq system, including operational mode, simulation settings,
/// and network configuration.
///
/// # Configuration Sections
///
/// - `schema_version`: Version of the configuration schema for compatibility
/// - `mode`: Operational mode (Production or Simulation)
/// - `simulation`: Simulation-specific parameters (only used in Simulation mode)
/// - `ports`: Network port assignments for all services
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
