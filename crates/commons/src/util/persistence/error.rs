//! Canonical persistence error types.
//!
//! This enum groups the common failure cases you might see when talking
//! to a database. Drivers (SQLite, Postgres, memory) convert their own
//! errors into these variants so services can handle them in a consistent,
//! easy‑to‑understand way.
//!
//! # Examples
//! ```
//! use commons::util::persistence::PersistenceError;
//! fn needs_value(v: Option<&str>) -> Result<(), PersistenceError> {
//!     v.map(|_| ()).ok_or(PersistenceError::NotFound)
//! }
//! assert!(matches!(needs_value(None), Err(PersistenceError::NotFound)));
//! ```

use thiserror::Error;

/// Persistence layer error.
///
/// # Variants
/// - `DeadlineExceeded`: the operation ran out of time
/// - `InvalidInput(String)`: bad input or schema mismatch
/// - `Unavailable(String)`: backend not reachable or not ready
/// - `Conflict(String)`: constraint violation or conflicting write
/// - `NotFound`: missing record for fetch/delete operations
/// - `Internal(String)`: unknown/other error with message
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
