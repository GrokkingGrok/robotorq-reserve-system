use thiserror::Error;

/// Errors occurring during startup/initialization that are not specific
/// to persistence or messaging (e.g. binding failures, config validation).
#[derive(Debug, Error)]
pub enum StartupError {
    /// Error binding to a network address or port.
    #[error("BindError: {0}")]
    Bind(String),

    /// Config validation or missing configuration values.
    #[error("Config validation failed: {0}")]
    Config(String),

    /// Other miscellaneous startup failures.
    #[error("Other startup error: {0}")]
    Other(String),
}
