//! Service abstractions and transport adapters.
//!
//! This module hosts HTTP server utilities and traits for exposing services
//! without coupling business logic to specific transports.
pub mod http;

// Do not glob re-export; access via `crate::services::robot_gateway`.