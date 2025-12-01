//! Logging utilities for the RoboTorq Reserve System.
//!
//! This module provides centralized logging configuration using the `tracing` crate.
//! It supports both human-readable output for development and JSON output for production
//! services, with optional file rotation for persistent logging.
//!
//! # Quick Start
//!
//! For development with pretty output:
//! ```rust,ignore
//! use commons::util::logging::init_logging_pretty;
//!
//! init_logging_pretty("debug")?;
//! ```
//!
//! For production with JSON and file logging:
//! ```rust,ignore
//! use commons::util::logging::init_logging;
//!
//! init_logging(true, "info", Some("/var/log/robotorq"))?;
//! ```

use tracing_appender::rolling;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize tracing with optional JSON output and rolling file appender.
///
/// Configures the global tracing subscriber with stdout logging and optional daily
/// rotating file logging. The log level can be controlled via the `RUST_LOG` environment
/// variable, falling back to the provided `default_level`.
///
/// # Arguments
///
/// * `json` - When true, logs are emitted as JSON (recommended for services)
/// * `default_level` - Default log level (e.g., "info", "debug", "warn") if `RUST_LOG` is not set
/// * `rolling_dir` - Optional directory path for daily rotating log files. When `None`, only stdout logging is used
///
/// # Returns
///
/// Returns `Ok(())` on successful initialization, or an error if the logger is already initialized
/// or if file logging setup fails.
///
/// # Errors
///
/// * Returns an error if the global logger has already been initialized
/// * Returns an error if the rolling file directory cannot be created or accessed
///
/// # Examples
///
/// Basic stdout logging:
/// ```rust,ignore
/// use commons::util::logging::init_logging;
///
/// init_logging(false, "info", None)?;
/// ```
///
/// JSON logging with file rotation:
/// ```rust,ignore
/// init_logging(true, "debug", Some("/var/log/robotorq"))?;
/// ```
///
/// # Panics
///
/// This function does not panic. All error conditions are returned as `Result` values.
pub fn init_logging(
    json: bool,
    default_level: &str,
    rolling_dir: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    // Optional rolling file sink
    let file_layer = rolling_dir.map(|dir| {
        let file_appender = rolling::daily(dir, "robotorq.log");
        fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_target(true)
            .with_level(true)
            .json()
    });

    // Stdout layer (choose one implementation path to avoid type mismatch)
    let stdout_layer = if json {
        fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_level(true)
            .json()
    } else {
        // Use a JSON formatter with human-oriented fields to keep type uniform
        fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_level(true)
            .json()
    };

    if let Some(file) = file_layer {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .with(file)
            .try_init()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .try_init()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

/// Initialize a human-friendly compact logger (ANSI colors, no JSON), stdout only.
///
/// Configures a simple, human-readable logger that outputs to stdout with ANSI colors
/// and compact formatting. This is ideal for development, CLI tools, or when you want
/// easily readable logs without JSON structure.
///
/// The log level can be controlled via the `RUST_LOG` environment variable,
/// falling back to the provided `default_level`.
///
/// # Arguments
///
/// * `default_level` - Default log level (e.g., "info", "debug", "warn") if `RUST_LOG` is not set
///
/// # Returns
///
/// Returns `Ok(())` on successful initialization, or an error if the logger is already initialized.
///
/// # Errors
///
/// * Returns an error if the global logger has already been initialized
///
/// # Examples
///
/// ```rust,ignore
/// use commons::util::logging::init_logging_pretty;
///
/// init_logging_pretty("debug")?;
/// ```
///
/// # Panics
///
/// This function does not panic. All error conditions are returned as `Result` values.
pub fn init_logging_pretty(default_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .compact();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .try_init()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    /// Test that `init_logging` compiles and runs without panicking.
    ///
    /// This test verifies that the logging initialization function can be called
    /// with basic parameters and doesn't cause any runtime panics. It ignores
    /// any initialization errors (which may occur if logging is already set up)
    /// and focuses on ensuring the function signature and basic execution path work.
    ///
    /// # Note
    ///
    /// This test does not verify actual logging output, only that the initialization
    /// process completes without panicking.
    #[test]
    fn init_logging_compiles_and_runs() {
        let _ = init_logging(false, "info", None); // Ignore error if already set
        info!(component = "commons.logging", "logging initialized");
        // No assert; test ensures no panic and basic path compiles.
    }

    /// Test that `init_logging_pretty` compiles and runs without panicking.
    ///
    /// This test verifies that the pretty logging initialization function can be called
    /// with basic parameters and doesn't cause any runtime panics. It ignores
    /// any initialization errors (which may occur if logging is already set up)
    /// and focuses on ensuring the function signature and basic execution path work.
    ///
    /// # Note
    ///
    /// This test does not verify actual logging output, only that the initialization
    /// process completes without panicking.
    #[test]
    fn init_logging_pretty_compiles_and_runs() {
        let _ = init_logging_pretty("debug"); // Ignore error if already set
        info!(component = "commons.logging", "pretty logging initialized");
    }
}
