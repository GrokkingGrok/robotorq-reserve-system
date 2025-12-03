use thiserror::Error;

/// Errors used by examples and small integration binaries.
#[derive(Debug, Error)]
pub enum ExampleError {
    /// Generic textual error wrapper for examples to avoid adding crate deps inside `commons`.
    #[error("{0}")]
    Other(String),
}
