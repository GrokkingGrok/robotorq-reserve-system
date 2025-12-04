//! Network Port Configuration
//!
//! This module defines network port assignments for all `RoboTorq` Reserve System services.
//! It provides a centralized configuration for service endpoints including the robot gateway,
//! metrics collection, and monitoring dashboards.
//!
//! # Service Ports
//!
//! The system uses dedicated ports for different services:
//! - **Robot Gateway**: Primary service for robot communication and batch processing
//! - **Metrics**: Prometheus metrics exposition for monitoring and alerting
//! - **Grafana**: Web dashboard for visualization and system monitoring
//!
//! # Port Assignment Strategy
//!
//! Ports are assigned to avoid conflicts with common services:
//! - Robot Gateway: 9000 (high port, avoids common service conflicts)
//! - Metrics: 8005 (distinct from default Prometheus port 9090)
//! - Grafana: 8015 (distinct from default Grafana port 3000)
//!
//! # Configuration Sources
//!
//! Port configuration can be loaded from:
//! - Environment variables (using uppercase names with `ROBOT_GATEWAY_PORT`, etc.)
//! - TOML configuration files
//! - Default values (fallback when configuration is missing)
//!
//! # Environment Variable Support
//!
//! Ports can be overridden via environment variables:
//! - `ROBOT_GATEWAY_PORT`: Override robot gateway port
//! - `METRICS_PORT`: Override metrics port
//! - `GRAFANA_PORT`: Override Grafana port

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Network port configuration for all `RoboTorq` services.
///
/// This struct defines the port assignments for all network services in the
/// `RoboTorq` system. Ports can be configured via environment variables or
/// configuration files, with sensible defaults provided.
///
/// # Environment Variables
///
/// All ports support environment variable override using the field names
/// converted to uppercase with underscores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortsConfig {
    /// Port for the Robot Gateway service.
    ///
    /// This is the primary service port where robots connect to submit
    /// work batches and receive system coordination. Defaults to 9000.
    #[serde(rename = "ROBOT_GATEWAY_PORT", default = "default_robot_gateway_port")]
    pub robot_gateway_port: u16,

    /// Port for Prometheus metrics exposition.
    ///
    /// Metrics are exposed on this port in Prometheus format for monitoring,
    /// alerting, and observability. Defaults to 8005.
    #[serde(rename = "METRICS_PORT", default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Port for the Grafana web dashboard.
    ///
    /// The Grafana dashboard provides visualization of system metrics,
    /// performance data, and operational status. Defaults to 8015.
    #[serde(rename = "GRAFANA_PORT", default = "default_grafana_port")]
    pub grafana_port: u16,
}

/// Returns the default port for the Robot Gateway service.
fn default_robot_gateway_port() -> u16 {
    9000
}

/// Returns the default port for Prometheus metrics exposition.
fn default_metrics_port() -> u16 {
    8005
}

/// Returns the default port for the Grafana dashboard.
fn default_grafana_port() -> u16 {
    8015
}

/// Load ports configuration from the repository default TOML file.
///
/// This function attempts to load port configuration from a default TOML file
/// located at `crates/commons/src/util/config/parameters/ports.toml`. If the
/// file is missing or invalid, it falls back to default port values.
///
/// # Behavior
///
/// - If the file exists and is valid: loads configuration and logs success
/// - If the file exists but is invalid: logs warning and uses defaults
/// - If the file doesn't exist: logs info and uses defaults
///
/// # Returns
///
/// Always returns a valid `PortsConfig` with either loaded values or defaults.
///
/// # Logging
///
/// This function logs its actions using the `commons.config` component:
/// - Success: when configuration is loaded successfully
/// - Warning: when configuration file exists but is invalid
/// - Info: when configuration file is missing (acceptable fallback)
pub fn load_ports_config_from_default() -> PortsConfig {
    let rel = "crates/commons/src/util/config/parameters/ports.toml";
    let p = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let path = p.join(rel);
    if let Ok(s) = std::fs::read_to_string(&path) {
        match toml::from_str::<PortsConfig>(&s) {
            Ok(cfg) => {
                info!(component = "commons.config", path = %path.display(), "loaded ports config");
                cfg
            }
            Err(e) => {
                warn!(component = "commons.config", path = %path.display(), error = %e, "failed to parse ports config; using defaults");
                PortsConfig {
                    robot_gateway_port: default_robot_gateway_port(),
                    metrics_port: default_metrics_port(),
                    grafana_port: default_grafana_port(),
                }
            }
        }
    } else {
        // Silent fallback with a small info: file missing is acceptable
        info!(component = "commons.config", path = %path.display(), "ports config not found; using defaults");
        PortsConfig {
            robot_gateway_port: default_robot_gateway_port(),
            metrics_port: default_metrics_port(),
            grafana_port: default_grafana_port(),
        }
    }
}
