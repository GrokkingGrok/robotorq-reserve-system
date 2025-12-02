use thiserror::Error;

/// Errors encountered during shutdown/cleanup.
#[derive(Debug, Error)]
pub enum ShutdownError {
    /// Failure releasing a resource (file, socket, client, etc.).
    #[error("ResourceReleaseError: {0}")]
    ResourceRelease(String),

    /// Timeout expired while waiting for shutdown completion.
    #[error("Timeout while shutting down: {0}")]
    Timeout(String),

    /// Other miscellaneous shutdown errors.
    #[error("Other shutdown error: {0}")]
    Other(String),
}
