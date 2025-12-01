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

pub mod batch_error;
pub mod config_error;
pub mod logging_error;
pub mod prometheus_error;
pub mod robot_error;
pub mod robot_gateway_error;
pub mod token_error;
pub mod triple_torq_error;

use batch_error::BatchError;
use config_error::ConfigError;
use logging_error::LoggingError;
use prometheus_error::PrometheusError;
use robot_error::RobotError;
use robot_gateway_error::RobotGatewayError;
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
}
