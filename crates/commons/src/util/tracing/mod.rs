//! Tracing propagation helpers.
//!
//! This small module provides neutral, transport-agnostic helpers for
//! injecting and extracting W3C trace context into simple key/value maps.
//!
//! The functions are deliberately minimal and dependency-free so they can be
//! used by message adapters (HTTP headers, NATS message headers) without
//! pulling heavy tracing or transport crates into modules that don't need
//! them. Concrete adapters should call these helpers to map trace context to
//! their transport headers and vice-versa.

pub mod propagation;

pub use propagation::{extract_trace_context, inject_trace_context};
