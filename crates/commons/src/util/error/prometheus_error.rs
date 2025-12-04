//! Prometheus Metrics Error Types
//!
//! This module defines error types related to Prometheus metrics collection
//! and exposition in the `RoboTorq` Reserve System. Metrics errors handle failures
//! in metric registration, updating, and serving.
//!
//! # Metrics Architecture
//!
//! The system uses Prometheus for monitoring and observability with:
//! - Counter metrics for events (batches captured, robots registered)
//! - Gauge metrics for current state (active robots, queue sizes)
//! - Histogram metrics for performance measurements
//! - Custom metrics for business logic tracking
//!
//! # Common Operations
//!
//! Metrics operations include:
//! - Registering metrics with the Prometheus registry
//! - Updating metric values
//! - Serving metrics via HTTP endpoints
//! - Handling metric collection failures

use thiserror::Error;

/// Errors that occur during Prometheus metrics operations.
///
/// This enum wraps Prometheus library errors to provide consistent error
/// handling throughout the `RoboTorq` metrics system.
#[derive(Debug, Error)]
pub enum PrometheusError {
    /// An error occurred in the underlying Prometheus library.
    ///
    /// This error wraps any error from the Prometheus crate, including:
    /// - Metric registration conflicts
    /// - Invalid metric names or labels
    /// - Registry operation failures
    /// - Encoding/decoding errors
    ///
    /// # Examples
    /// ```rust
    /// # use commons::util::error::prometheus_error::PrometheusError;
    /// # use prometheus::Error;
    /// // This would typically come from prometheus operations
    /// // let prometheus_err = prometheus::Error::Msg("metric already registered".to_string());
    /// // let error = PrometheusError::Inner(prometheus_err);
    /// ```
    #[error("prometheus error: {0}")]
    Inner(#[from] prometheus::Error),
}
