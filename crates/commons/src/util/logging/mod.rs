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

use std::future::Future;
use std::sync::Once;
use std::time::{Duration, Instant};

use tracing_appender::rolling;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};
// Note: earlier attempts to implement a custom `FormatEvent` used private
// `tracing-subscriber` internals and caused fragile, version-dependent errors.
// We avoid custom formatter implementations here and rely on the stable
// `fmt::layer().json()` option when JSON output is desired.
// Note: we avoid importing private `tracing-subscriber` internals or unused
// helper crates here. The module uses the stable `fmt::layer().json()` for
// JSON output and middleware-inserted trace context for correlation.
use tokio::task::JoinHandle;

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
/// Production-oriented tracing initializer (idempotent).
///
/// This is the primary entry point for services that need structured logging
/// and optional file rotation. It's idempotent — calling it multiple times
/// is safe and subsequent calls are no-ops.
pub fn init_prod_tracing(
    json: bool,
    default_level: &str,
    rolling_dir: Option<&str>,
    otlp: Option<OtlpConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    static INIT: Once = Once::new();
    let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());

    INIT.call_once(|| {
        // Build env filter
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

        // Note: avoiding a custom FormatEvent implementation and relying on
        // the stable `fmt::layer().json()` option. Trace/span correlation is
        // handled via middleware-inserted `TraceContext` placed into request
        // extensions, which keeps formatting and tracing concerns decoupled.

        // Apply the subscriber. Build the stdout layer (and optional file layer)
        // inline per-formatter so types remain consistent.
        let init_res = if json {
            // JSON stdout and optional JSON file
            let file_layer = rolling_dir.map(|dir| {
                let file_appender = rolling::daily(dir, "robotorq.log");
                fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .with_target(true)
                    .with_level(true)
                    .json()
            });

            if let Some(file) = file_layer {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().with_ansi(false).with_target(true).with_level(true).json())
                    .with(file)
                    .try_init()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().with_ansi(false).with_target(true).with_level(true).json())
                    .try_init()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            }
        } else {
            // Human-friendly compact stdout and optional compact file
            let file_layer = rolling_dir.map(|dir| {
                let file_appender = rolling::daily(dir, "robotorq.log");
                fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .with_target(true)
                    .with_level(true)
                    .compact()
            });

            if let Some(file) = file_layer {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().with_ansi(true).with_target(true).with_level(true).compact())
                    .with(file)
                    .try_init()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer().with_ansi(true).with_target(true).with_level(true).compact())
                    .try_init()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            }
        };

        if let Err(e) = init_res {
            result = Err(e);
            return;
        }

        if otlp.is_some() {
            // OTLP support is feature-gated and requires careful dependency configuration
            // for the opentelemetry/runtime and exporter features. The full OTLP pipeline
            // installation was deferred to avoid fragile version/runtime coupling in CI.
            // If you need OTLP export, implement a pipeline using `opentelemetry-otlp`
            // and the matching runtime feature flags (e.g., `rt-tokio`) in `Cargo.toml`.
            tracing::warn!("OTLP configured but OTLP pipeline installation is disabled in this build; enable and implement pipeline to export traces");
        }
    });

    result
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
/// Test-friendly logging initializer (human, compact, idempotent).
pub fn init_test_logging(default_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    static INIT_PRETTY: Once = Once::new();
    let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());

    INIT_PRETTY.call_once(|| {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
        let stdout_layer = fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_level(true)
            .compact();
        let init_res = tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .try_init()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
        if let Err(e) = init_res {
            result = Err(e);
        }
    });

    result
}

/// Backwards-compatible convenience wrapper to match older API name.
pub fn init_logging(
    json: bool,
    default_level: &str,
    rolling_dir: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    init_prod_tracing(json, default_level, rolling_dir, None)
}

/// Simple configuration object for OTLP exporter; currently a placeholder
/// that is ignored unless the `otlp` cargo feature is enabled.
#[derive(Clone, Debug)]
pub struct OtlpConfig {
    /// Optional OTLP collector endpoint override (e.g., `http://collector:4317`).
    /// When `None`, the default exporter endpoint is used.
    pub endpoint: Option<String>,
}

/// Log a warning if a lock acquisition waited longer than `threshold`.
pub fn log_if_waited(mutex_name: &str, start: Instant, threshold: Duration) {
    let waited = start.elapsed();
    if waited > threshold {
        tracing::warn!(mutex = mutex_name, waited_ms = %waited.as_millis(), "lock waited longer than threshold");
    }
}

/// Spawn a task attached to a short-lived tracing span.
pub fn spawn_traced<F, T>(name: &'static str, fut: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let span = tracing::span!(tracing::Level::INFO, "task", name = name);
    tokio::spawn(async move {
        let _enter = span.enter();
        fut.await
    })
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
        let _ = init_test_logging("debug"); // Ignore error if already set
        info!(component = "commons.logging", "pretty logging initialized");
    }
}
