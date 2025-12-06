//! RoboTorq Reserve System — Commons Crate
//!
//! Overview
//! - This crate is the entry point to the RoboTorq ecosystem: shared types,
//!   configuration, persistence primitives, metrics, and service interfaces.
//! - It is designed to be readable and safe, with compile‑checked examples and
//!   clear invariants so teams can build on it confidently.
//!
//! Economic Invariants (must always hold)
//! - 1 `TokenTorqIngot` = 3,600 `JouleTorqOre` units
//! - 1 `RoboTorq Certificate` = 1,000 ingots = 3,600,000 Ore units
//!
//! Architecture Map
//! - Types: `types::ids`, `types::token`, `types::robot`, `types::triple_torq`
//! - Persistence: `util::persistence` (Context deadlines, drivers, errors, spans)
//! - Metrics: `util::metrics` (Prometheus adapter + helpers)
//! - Config: `util::config` (observability, economic, services, security)
//! - Services: `services::robotorq_service` (HTTP server, readiness, middleware)
//!
//! Quick Start
//! - Persistence: create a driver via `util::persistence::make_driver`, then use
//!   `with_db_span` and `Context::with_deadline_from_now` for traceable, time‑bounded ops.
//! - Readiness: use service handlers `/readyz` (cached) and `/readyz-strict` (per‑request)
//!   to gate start‑up and traffic.
//! - Metrics: construct `util::metrics::PrometheusRegistry`, apply
//!   `services::robotorq_service::middleware::HttpMetricsLayer` to Axum routers, and
//!   expose `GET /metrics` with your registry text exporter.
//!
//! Development Notes
//! - SQLite‑first: Postgres is opt‑in via features; migrations are structured and idempotent.
//! - Observability: prefer tracing spans for DB/HTTP; logging is available for human‑readable events.
//! - Doctests: examples are compile‑checked using `no_run` or `ignore` to keep docs actionable.
//!
//! Where to go next
//! - Persistence overview: `commons::util::persistence`
//! - Metrics overview: `commons::util::metrics`
//! - Service scaffolding: `commons::services::robotorq_service`
//! - Economic config and invariants: `commons::util::config::economic`
//!
//! Use this crate to build service implementations while keeping business logic
//! decoupled from HTTP and transport layers.
#![allow(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::default_trait_access,
    clippy::unnecessary_wraps,
    clippy::implicit_hasher
)]
pub mod services;
pub mod types;
pub mod util;

pub use types::Robot;
pub use types::Token;
pub use types::UnmappedOreBatch;
pub use types::ids::{RobotId, TokenId, TripleTorqId, UnmappedOreBatchId};
pub use types::triple_torq::TripleTorq;
