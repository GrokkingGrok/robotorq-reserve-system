//! Canonical persistence error types.
//!
//! Use this error to normalize backend-specific failures across drivers.

use thiserror::Error;

/// Persistence layer error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PersistenceError {
    /// Operational timeout or deadline exceeded.
    #[error("deadline exceeded")]
    DeadlineExceeded,
    /// Invalid input or schema mismatch.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// Backend unavailable or not ready.
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    /// Conflict or constraint violation.
    #[error("conflict: {0}")]
    Conflict(String),
    /// Not found for fetch/delete.
    #[error("not found")]
    NotFound,
    /// Unknown or wrapped error.
    #[error("internal error: {0}")]
    Internal(String),
}
