//! Configuration Error Types
//!
//! This module defines error types related to configuration parsing, validation,
//! and loading in the RoboTorq Reserve System. Configuration errors handle issues
//! with system configuration files, environment variables, and runtime settings.
//!
//! # Configuration Sources
//!
//! The system supports multiple configuration sources:
//! - TOML configuration files (eg. `ports.toml`)
//! - Environment variables
//! - Command-line arguments
//! - Runtime configuration updates
//!
//! # Validation Rules
//!
//! Configuration validation ensures:
//! - Required fields are present
//! - Values are within acceptable ranges
//! - Dependencies between settings are satisfied
//! - File paths exist and are accessible

use thiserror::Error;

/// Errors that occur during configuration operations.
///
/// These errors represent problems with loading, parsing, or validating
/// the RoboTorq system configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration contains invalid or malformed data.
    ///
    /// This error occurs when configuration parsing fails or when validation
    /// rules are violated. The error message provides details about what
    /// specifically is invalid.
    ///
    /// # Causes
    /// - Malformed TOML syntax
    /// - Invalid value ranges (e.g., negative port numbers)
    /// - Missing required configuration fields
    /// - Inconsistent configuration settings
    ///
    /// # Examples
    /// ```rust
    /// # use commons::util::error::config_error::ConfigError;
    /// let error = ConfigError::Invalid("Port number must be between 1 and 65535".to_string());
    /// ```
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}