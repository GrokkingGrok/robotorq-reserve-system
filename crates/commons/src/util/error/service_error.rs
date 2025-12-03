use thiserror::Error;

/// Service-level errors used internally by commons service helpers.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Generic wrapper for errors surfaced by lower-level operations.
    #[error("{0}")]
    Other(String),
}

impl From<&str> for ServiceError {
    fn from(s: &str) -> Self {
        ServiceError::Other(s.to_string())
    }
}
