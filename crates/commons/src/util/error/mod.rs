//! Error Types and Invariants
//!
//! This module provides a comprehensive error handling system for the RoboTorq
//! Reserve System. It defines specific error types for different subsystems and
//! exports domain-specific errors and the internal `ServiceError` used by
//! commons service helpers.
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
//! The codebase models economic and cryptographic invariants via domain errors
//! (e.g., `TokenError`, `TripleTorqError`, `RobotError`). Over time these domain
//! errors replace any single unified error type so callers can handle failures
//! more precisely.
//!
//! Migration note:
//! - `InvariantError` has been removed; internal modules now return domain-specific
//!   errors or the `ServiceError` helper at service boundaries. Consumers should
//!   prefer domain errors where possible and map to `ServiceError` at HTTP/service
//!   boundaries if a unified service-level error is required.

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
pub mod service_error;
pub use service_error::ServiceError;

// Re-export domain errors for convenience. Callers should prefer domain-specific
// error types for fine-grained handling.
pub use batch_error::BatchError;
pub use config_error::ConfigError;
pub use logging_error::LoggingError;
pub use messaging_error::MessagingError;
pub use persistence_error::PersistenceError;
pub use prometheus_error::PrometheusError;
pub use robot_error::RobotError;
pub use robot_gateway_error::RobotGatewayError;
pub use shutdown_error::ShutdownError;
pub use startup_error::StartupError;
pub use token_error::TokenError;
pub use triple_torq_error::TripleTorqError;
