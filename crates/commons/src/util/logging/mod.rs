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

// Note: OTLP / OpenTelemetry exporter support is feature-gated in the `commons` crate.
// To enable the optional OTLP dependencies in downstream crates or examples, enable
// the `otlp` feature on the `commons` dependency in `Cargo.toml` like this:
//
// ```toml
// [dependencies]
// commons = { path = "../commons", features = ["otlp"], default-features = false }
// ```
//
// Alternatively, examples can use `opentelemetry` / `opentelemetry-otlp` /
// `tracing-opentelemetry` directly as demonstrated in `crates/examples/src/bin/emit_traces.rs`.

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

        // Build and install the subscriber registry in branch to avoid mixing concrete
        // `fmt::layer()` return types (JsonFields vs DefaultFields). Each branch builds
        // its own layers and installs the global subscriber independently.
        if json {
            let fmt_layer = fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .json();

            // Build and install the subscriber. We avoid returning different concrete
            // types from a single `if` expression by splitting the logic into nested
            // branches: one for whether a rolling file appender is present, and one
            // for whether OTLP is configured. Each branch constructs a concrete
            // subscriber and calls `.try_init()` independently.
            if let Some(dir) = rolling_dir {
                let file_appender = rolling::daily(dir, "robotorq.log");
                let file_layer = fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .with_target(true)
                    .with_level(true)
                    .json();

                let registry = tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(file_layer);

                if let Some(cfg) = otlp.clone() {
                    #[cfg(feature = "otlp")]
                    {
                        use opentelemetry_sdk::trace as sdktrace;
                        use opentelemetry_otlp::WithExportConfig;

                        let exporter_builder = opentelemetry_otlp::new_exporter().tonic();
                        let exporter = if let Some(ep) = cfg.endpoint {
                            exporter_builder.with_endpoint(ep)
                        } else {
                            exporter_builder
                        };

                        let tracer = opentelemetry_otlp::new_pipeline()
                            .tracing()
                            .with_exporter(exporter)
                            .with_trace_config(sdktrace::Config::default())
                            .install_batch(opentelemetry_sdk::runtime::Tokio)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

                        match tracer {
                            Ok(tracer) => {
                                tracing::debug!("OTLP tracer created successfully");
                                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                                let init_res = registry.with(otel_layer).try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                                if let Err(e) = init_res {
                                    result = Err(e);
                                    return;
                                }
                                tracing::debug!("tracing subscriber initialized with OTLP exporter");
                            }
                            Err(e) => {
                                result = Err(e);
                            }
                        }
                    }

                    #[cfg(not(feature = "otlp"))]
                    {
                        tracing::warn!("OTLP configured but crate compiled without `otlp` feature; enable `otlp` feature to export traces");
                        let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                        if let Err(e) = init_res {
                            result = Err(e);
                            return;
                        }
                        tracing::debug!("tracing subscriber initialized without OTLP exporter");
                    }
                } else {
                    let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                    if let Err(e) = init_res {
                        result = Err(e);
                    }
                }
            } else {
                // No rolling file appender: install subscriber with stdout-only fmt layer.
                let registry = tracing_subscriber::registry().with(env_filter).with(fmt_layer);

                if let Some(cfg) = otlp.clone() {
                    #[cfg(feature = "otlp")]
                    {
                        use opentelemetry_sdk::trace as sdktrace;
                        use opentelemetry_otlp::WithExportConfig;

                        let exporter_builder = opentelemetry_otlp::new_exporter().tonic();
                        let exporter = if let Some(ep) = cfg.endpoint {
                            exporter_builder.with_endpoint(ep)
                        } else {
                            exporter_builder
                        };

                        let tracer = opentelemetry_otlp::new_pipeline()
                            .tracing()
                            .with_exporter(exporter)
                            .with_trace_config(sdktrace::Config::default())
                            .install_batch(opentelemetry_sdk::runtime::Tokio)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

                        match tracer {
                            Ok(tracer) => {
                                tracing::debug!("OTLP tracer created successfully");
                                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                                let init_res = registry.with(otel_layer).try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                                if let Err(e) = init_res {
                                    result = Err(e);
                                    return;
                                }
                                tracing::debug!("tracing subscriber initialized with OTLP exporter");
                            }
                            Err(e) => {
                                result = Err(e);
                            }
                        }
                    }

                    #[cfg(not(feature = "otlp"))]
                    {
                        tracing::warn!("OTLP configured but crate compiled without `otlp` feature; enable `otlp` feature to export traces");
                        let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                        if let Err(e) = init_res {
                            result = Err(e);
                            return;
                        }
                        tracing::debug!("tracing subscriber initialized without OTLP exporter");
                    }
                } else {
                    let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                    if let Err(e) = init_res {
                        result = Err(e);
                    }
                }
            }
        } else {
            let fmt_layer = fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_level(true)
                .compact();

            // Nested branch approach for compact formatter (mirror of JSON branch).
            if let Some(dir) = rolling_dir {
                let file_appender = rolling::daily(dir, "robotorq.log");
                let file_layer = fmt::layer()
                    .with_writer(file_appender)
                    .with_ansi(false)
                    .with_target(true)
                    .with_level(true)
                    .compact();

                let registry = tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(file_layer);

                if let Some(cfg) = otlp.clone() {
                    #[cfg(feature = "otlp")]
                    {
                        use opentelemetry_sdk::trace as sdktrace;
                        use opentelemetry_otlp::WithExportConfig;

                        let exporter_builder = opentelemetry_otlp::new_exporter().tonic();
                        let exporter = if let Some(ep) = cfg.endpoint {
                            exporter_builder.with_endpoint(ep)
                        } else {
                            exporter_builder
                        };

                        let tracer = opentelemetry_otlp::new_pipeline()
                            .tracing()
                            .with_exporter(exporter)
                            .with_trace_config(sdktrace::Config::default())
                            .install_batch(opentelemetry_sdk::runtime::Tokio)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

                        match tracer {
                            Ok(tracer) => {
                                tracing::debug!("OTLP tracer created successfully");
                                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                                let init_res = registry.with(otel_layer).try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                                if let Err(e) = init_res {
                                    result = Err(e);
                                    return;
                                }
                                tracing::debug!("tracing subscriber initialized with OTLP exporter");
                            }
                            Err(e) => {
                                result = Err(e);
                            }
                        }
                    }

                    #[cfg(not(feature = "otlp"))]
                    {
                        tracing::warn!("OTLP configured but crate compiled without `otlp` feature; enable `otlp` feature to export traces");
                        let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                        if let Err(e) = init_res {
                            result = Err(e);
                            return;
                        }
                        tracing::debug!("tracing subscriber initialized without OTLP exporter");
                    }
                } else {
                    let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                    if let Err(e) = init_res {
                        result = Err(e);
                    }
                }
            } else {
                let registry = tracing_subscriber::registry().with(env_filter).with(fmt_layer);

                if let Some(cfg) = otlp.clone() {
                    #[cfg(feature = "otlp")]
                    {
                        use opentelemetry_sdk::trace as sdktrace;
                        use opentelemetry_otlp::WithExportConfig;

                        let exporter_builder = opentelemetry_otlp::new_exporter().tonic();
                        let exporter = if let Some(ep) = cfg.endpoint {
                            exporter_builder.with_endpoint(ep)
                        } else {
                            exporter_builder
                        };

                        let tracer = opentelemetry_otlp::new_pipeline()
                            .tracing()
                            .with_exporter(exporter)
                            .with_trace_config(sdktrace::Config::default())
                            .install_batch(opentelemetry_sdk::runtime::Tokio)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

                        match tracer {
                            Ok(tracer) => {
                                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                                let init_res = registry.with(otel_layer).try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                                if let Err(e) = init_res {
                                    result = Err(e);
                                }
                            }
                            Err(e) => {
                                result = Err(e);
                            }
                        }
                    }

                    #[cfg(not(feature = "otlp"))]
                    {
                        tracing::warn!("OTLP configured but crate compiled without `otlp` feature; enable `otlp` feature to export traces");
                        let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                        if let Err(e) = init_res {
                            result = Err(e);
                            return;
                        }
                    }
                } else {
                    let init_res = registry.try_init().map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
                    if let Err(e) = init_res {
                        result = Err(e);
                    }
                }
            }
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

/// Log a diagnostic message when an operation waited longer than `threshold`.
///
/// This helper is intended to centralize the policy for emitting diagnostics
/// when asynchronous locks or tasks observe unexpected wait durations. The
/// `start` argument should be the instant when the wait began (e.g., the time
/// a lock acquisition was requested). If the observed wait exceeds `threshold`
/// a `warn!` is emitted; otherwise a `debug!` is emitted for diagnostic
/// visibility.
pub fn log_if_waited(key: &str, start: Instant, threshold: Duration) {
    let waited = Instant::now().duration_since(start);
    if waited > threshold {
        tracing::warn!(key = %key, waited_ms = %waited.as_millis(), "operation waited longer than threshold");
    } else {
        tracing::debug!(key = %key, waited_ms = %waited.as_millis(), "operation wait time");
    }
}

/// Flush and shutdown the global tracer provider if OTLP support is enabled.
///
/// This is a no-op when the `otlp` feature is not enabled for the `commons`
/// crate. Call this during graceful shutdown to best-effort flush exporter
/// buffers so spans are exported before process exit.
pub fn shutdown_tracer_provider() {
    #[cfg(feature = "otlp")]
    {
        opentelemetry::global::shutdown_tracer_provider();
    }
}

/// Spawn a background task with an attached tracing span.
///
/// The created span will have the static name `background_task` and include
/// the provided `task` field for easier identification in logs/traces.
pub fn spawn_traced<Fut, T>(name: &str, fut: Fut) -> tokio::task::JoinHandle<T>
where
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let span = tracing::info_span!("background_task", task = %name);
    tokio::spawn(async move {
        let _enter = span.enter();
        fut.await
    })
}

// Note: OTLP support is optional and only active when the `otlp` cargo
// feature is enabled for the `commons` crate. When enabled, callers can pass
// an `OtlpConfig` to `init_prod_tracing` to install an OTLP exporter and the
// `tracing-opentelemetry` layer. If the feature is not enabled but an
// `OtlpConfig` is provided, a runtime warning is emitted and no exporter is
// installed.
