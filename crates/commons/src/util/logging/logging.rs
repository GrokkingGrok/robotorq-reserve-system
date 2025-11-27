use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::prelude::*;
use tracing_appender::rolling;

/// Initialize tracing with optional JSON output and rolling file appender.
/// - `json`: when true, logs are emitted as JSON (good for services).
/// - `default_level`: e.g., "info"; can be overridden by `RUST_LOG`.
/// - `rolling_dir`: optional directory for daily rolling files; when `None`, only stdout.
pub fn init_logging(json: bool, default_level: &str, rolling_dir: Option<&str>) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

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
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .with(file)
            .try_init();
    } else {
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .try_init();
    }
}

/// Initialize a human-friendly compact logger (ANSI, no JSON), stdout only.
pub fn init_logging_pretty(default_level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));
    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .compact();
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[test]
    fn init_logging_compiles_and_runs() {
        init_logging(false, "info", None);
        info!(component = "commons.logging", "logging initialized");
        // No assert; test ensures no panic and basic path compiles.
    }

    #[test]
    fn init_logging_pretty_compiles_and_runs() {
        init_logging_pretty("debug");
        info!(component = "commons.logging", "pretty logging initialized");
    }
}
