//! Service-level error adapter for commons service helpers.
//!
//! This module exposes `ServiceError`, a deliberately small and easily
//! mappable error type intended for use at transport or adapter boundaries
//! (for example: HTTP handlers, RPC adapters, CLI entrypoints). Business
//! logic and domain layers should prefer domain-specific error types
//! (e.g. `TokenError`, `RobotError`) and only translate into `ServiceError`
//! when crossing a service boundary.
//!
//! Rationale
//! - Keeping a compact service error type simplifies translation to HTTP
//!   status codes, structured error payloads, and stable logs/metrics.
//! - Domain errors remain expressive for internal handling and testing while
//!   `ServiceError` keeps transport concerns decoupled.
//!
//! Examples
//! ```rust
//! use commons::util::error::ServiceError;
//!
//! // business logic returns domain errors
//! // fn do_work() -> Result<(), TokenError> { ... }
//!
//! // handler maps domain->service for HTTP responses
//! // fn handle() -> Result<String, ServiceError> {
//! //     do_work().map_err(|e| ServiceError::from(e.to_string()))?;
//! //     Ok("ok".to_string())
//! // }
//! ```
//!
//! Mapping guidance (at the transport layer)
//! - Domain `NotFound` -> HTTP 404 with `ServiceError::Other` containing context.
//! - Domain `Validation` -> HTTP 400 with a structured message.
//! - Domain/Internal errors -> HTTP 500 and record full domain error in logs.

use thiserror::Error;

/// Small service-level error type for transport/adaptor layers.
///
/// The enum is intentionally minimal — it wraps a single string message which
/// should contain human-friendly context and, when appropriate, a short
/// internal error code. Use domain-specific errors inside business logic and
/// map them to `ServiceError` when returning from handlers or exposing to
/// external clients.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Generic wrapper for errors surfaced by lower-level operations.
    ///
    /// This variant is useful for adapters and examples where a compact,
    /// serializable error representation is required. Prefer mapping domain
    /// errors into more structured responses at the transport layer.
    #[error("{0}")]
    Other(String),
}

/// Create a `ServiceError` from a `&str` by cloning into an owned message.
impl From<&str> for ServiceError {
    fn from(s: &str) -> Self {
        ServiceError::Other(s.to_string())
    }
}

/// Convenience conversion from `String` into `ServiceError`.
impl From<String> for ServiceError {
    fn from(s: String) -> Self {
        ServiceError::Other(s)
    }
}
