//! Error Types and Invariants
//!
//! This module provides a comprehensive error handling system for the RoboTorq
//! Reserve System. It defines specific error types for different subsystems and
//! provides a unified `InvariantError` type for consistent error propagation.
//!
//! # Error Hierarchy
//!
//! The error system is organized into specialized modules:
//! - `token_error`: Token validation and energy accounting errors
//! - `batch_error`: Batch processing and aggregation errors
//! - `robot_error`: Robot configuration and validation errors
//! - `robot_gateway_error`: Gateway service operation errors
//! - `config_error`: Configuration parsing and validation errors
//! - `triple_torq_error`: TripleTorq balance management errors
//! - `logging_error`: Logging system initialization errors
//! - `prometheus_error`: Metrics collection and exposition errors
//!
//! # Invariant Preservation
//!
//! All errors in this module represent violations of system invariants that
//! must be preserved for economic and cryptographic integrity:
//! - Energy conservation laws
//! - Economic unit relationships
//! - Cryptographic proof requirements
//! - System consistency constraints
//!
//! # Error Propagation
//!
//! The `InvariantError` enum provides transparent error conversion from all
//! subsystem errors, allowing consistent error handling throughout the codebase.
//!
//! Migration note:
//! - Internal modules are being migrated to return domain-specific, strongly-typed
//!   errors (e.g., `TokenError`, `TripleTorqError`, `RobotError`).
//! - To preserve backwards compatibility, public-facing APIs and traits currently
//!   continue to use `InvariantError` as a unified error type; `InvariantError`
//!   includes `#[from]` conversions so domain errors convert automatically at
//!   crate boundaries. Over time the project will reduce reliance on the
//!   unified error and prefer domain-specific errors internally.

pub mod batch_error;
pub mod config_error;
/// Example-focused errors used by small integration binaries and examples.
pub mod example_error;
pub mod logging_error;
/// Messaging subsystem errors (NATS, JetStream, publish/subscribe errors).
pub mod messaging_error;
/// Persistence errors (DB pool, query, migration related errors).
pub mod persistence_error;
pub mod prometheus_error;
pub mod robot_error;
pub mod robot_gateway_error;
/// Shutdown and cleanup errors encountered during graceful teardown.
pub mod shutdown_error;
/// Errors during general startup and initialization (binding, config validation).
pub mod startup_error;
pub mod token_error;
pub mod triple_torq_error;
pub use example_error::ExampleError;
/// Service-level helper errors used internally by commons service utilities.
pub mod service_error;
pub use service_error::ServiceError;

use batch_error::BatchError;
use config_error::ConfigError;
use logging_error::LoggingError;
use messaging_error::MessagingError;
use persistence_error::PersistenceError;
use prometheus_error::PrometheusError;
use robot_error::RobotError;
use robot_gateway_error::RobotGatewayError;
use shutdown_error::ShutdownError;
use startup_error::StartupError;
use thiserror::Error;
use token_error::TokenError;
use triple_torq_error::TripleTorqError;

/// Unified error type for all system invariant violations.
///
/// This enum provides a single error type that can represent any invariant
/// violation in the RoboTorq system. It uses transparent error conversion
/// to preserve the original error context while providing a consistent
/// interface for error handling.
///
/// # Usage
///
/// ```rust
/// # use commons::util::error::InvariantError;
/// # use commons::util::error::token_error::TokenError;
/// // Errors automatically convert
/// let token_error = TokenError::ZeroJoules(0);
/// let invariant_error: InvariantError = token_error.into();
/// ```
///
/// # Error Categories
///
/// - `Token`: Token validation and energy accounting violations
/// - `Batch`: Batch processing and aggregation invariant violations
/// - `Robot`: Robot configuration and specification violations
/// - `Gateway`: Robot Gateway service operation failures
/// - `Config`: Configuration parsing and validation errors
/// - `TripleTorq`: TripleTorq balance management violations
/// - `Logging`: Logging system initialization failures
/// - `Metrics`: Prometheus metrics operation failures
#[derive(Debug, Error)]
pub enum InvariantError {
    /// Token-related invariant violation.
    #[error(transparent)]
    Token(#[from] TokenError),

    /// Batch processing invariant violation.
    #[error(transparent)]
    Batch(#[from] BatchError),

    /// Robot configuration invariant violation.
    #[error(transparent)]
    Robot(#[from] RobotError),

    /// Robot Gateway operation error.
    #[error(transparent)]
    Gateway(#[from] RobotGatewayError),

    /// Configuration validation error.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// TripleTorq balance invariant violation.
    #[error(transparent)]
    TripleTorq(#[from] TripleTorqError),

    /// Logging system initialization error.
    #[error(transparent)]
    Logging(#[from] LoggingError),

    /// Metrics collection error.
    #[error(transparent)]
    Metrics(#[from] PrometheusError),

    /// Startup and initialization failures (external systems, bindings)
    #[error(transparent)]
    Startup(#[from] StartupError),

    /// Persistence layer errors (DB pools, queries)
    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    /// Messaging subsystem errors (NATS, JetStream, connectivity)
    #[error(transparent)]
    Messaging(#[from] MessagingError),

    /// Shutdown and cleanup failures
    #[error(transparent)]
    Shutdown(#[from] ShutdownError),
    /// Example / sample binaries errors (examples crate)
    #[error(transparent)]
    Example(#[from] ExampleError),
    /// Service-level errors for rich internal handling
    #[error(transparent)]
    Service(#[from] ServiceError),
}
