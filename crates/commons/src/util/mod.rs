//! Utilities for configuration, logging, metrics, schema versions, and timekeeping.
//!
//! These helpers provide common functionality used across services and types,
//! including error types, hashing helpers, and Prometheus metrics integration.
pub mod config;
pub mod hashing;
pub mod logging;
pub mod metrics;
pub mod schema;
pub mod timekeeping;
pub mod error;