//! Library exports for integration tests and external tooling.
//!
//! Re-exports internal modules so test crates and auxiliary scripts can access
//! configuration loaders, state managers, HTTP types, and registry utilities
//! without relying on binary-only interfaces.

pub mod config;
pub mod contract_state;
pub mod crypto;
pub mod http_api;
pub mod jtu_hasher;
pub mod jtu_storage;
pub mod metrics;
pub mod printer_registry;
pub mod printer_handlers;
