//! Logging System Error Types
//!
//! This module defines error types related to the logging system initialization
//! and configuration in the RoboTorq Reserve System. Logging errors handle failures
//! during the setup of structured logging, file output, and tracing infrastructure.
//!
//! # Logging Architecture
//!
//! The system uses the `tracing` ecosystem for structured logging with support for:
//! - JSON-formatted output for production
//! - Human-readable output for development
//! - File-based logging with rotation
//! - Multiple log levels and filtering
//!
//! # Initialization Process
//!
//! Logging initialization involves:
//! - Setting up tracing subscribers
//! - Configuring output formats (JSON vs pretty)
//! - Establishing file writers and rotation
//! - Setting log levels and filters

use thiserror::Error;

/// Errors that occur during logging system operations.
///
/// These errors represent failures in the logging infrastructure setup
/// and configuration that prevent proper system observability.
#[derive(Debug, Error)]
pub enum LoggingError {
    /// Logging system initialization failed.
    ///
    /// This error occurs when the tracing subscriber cannot be initialized
    /// or configured properly. The error message contains details about
    /// the specific initialization failure.
    ///
    /// # Causes
    /// - Invalid logging configuration
    /// - File permission issues for log output
    /// - Tracing subscriber conflicts
    /// - Environment variable parsing errors
    ///
    /// # Examples
    /// ```rust
    /// # use commons::util::error::logging_error::LoggingError;
    /// let error = LoggingError::InitFailed("Failed to create log file".to_string());
    /// ```
    #[error("logging initialization failed: {0}")]
    InitFailed(String),
}

impl From<&str> for LoggingError {
    fn from(s: &str) -> Self {
        Self::InitFailed(s.to_string())
    }
}

impl From<String> for LoggingError {
    fn from(s: String) -> Self {
        Self::InitFailed(s)
    }
}

impl From<Box<dyn std::error::Error>> for LoggingError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Self::InitFailed(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that LoggingError can be created from a &str.
    #[test]
    fn test_from_str() {
        let error: LoggingError = "test message".into();
        match error {
            LoggingError::InitFailed(msg) => assert_eq!(msg, "test message"),
        }
    }

    /// Test that LoggingError can be created from a String.
    #[test]
    fn test_from_string() {
        let error: LoggingError = "test message".to_string().into();
        match error {
            LoggingError::InitFailed(msg) => assert_eq!(msg, "test message"),
        }
    }

    /// Test that LoggingError can be created from a Box<dyn std::error::Error>.
    #[test]
    fn test_from_box_dyn_error() {
        let original_error = std::io::Error::new(std::io::ErrorKind::Other, "io error");
        let boxed_error: Box<dyn std::error::Error> = Box::new(original_error);
        let error: LoggingError = boxed_error.into();
        match error {
            LoggingError::InitFailed(msg) => assert!(msg.contains("io error")),
        }
    }
}
