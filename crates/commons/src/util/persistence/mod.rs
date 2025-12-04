//! Commons persistence module (Sprint 1 scaffolding)
//!
//! Provides minimal, transport-agnostic building blocks for persistence:
//! - `Context`: lightweight trace carrier + optional monotonic deadline
//! - `PersistenceError`: canonical error taxonomy
//! - `DbAttributes`: structured attributes for DB spans
//! - `with_db_span`: helper to wrap an async op with standardized tracing
//! - `PersistenceDriver`: tiny trait for health readiness integration
//!
//! This is design scaffolding for Phase 2.1 Sprint 1. Implementations
//! (memory/sqlite/postgres/migrations) land in later sprints.
//!
//! Doctest (shape only):
//! ```
//! use commons::util::persistence::{Context, DbAttributes, PersistenceError, with_db_span};
//!
//! # async fn example(ctx: &Context) -> Result<(), PersistenceError> {
//! let attrs = DbAttributes {
//!     driver: Some("sqlite".to_string()),
//!     op: Some("select".to_string()),
//!     entity: Some("system".to_string()),
//!     statement: Some("select 1".to_string()),
//! };
//!
//! let _res: Result<(), PersistenceError> = with_db_span(ctx, &attrs, async { Ok::<_, PersistenceError>(()) }).await;
//! Ok(())
//! # }
//! ```

mod context;
mod error;
mod memory;
mod span;
#[cfg(feature = "persistence")]
pub mod sqlite;
#[cfg(feature = "persistence-postgres")]
pub mod backends {
    pub mod postgres;
}
mod test_helpers;
mod traits;

#[cfg(feature = "persistence-postgres")]
pub use backends::postgres::PostgresDriver;
pub use context::Context;
pub use error::PersistenceError;
pub use memory::InMemoryDriver;
pub use span::{DbAttributes, with_db_span};
#[cfg(feature = "persistence")]
pub use sqlite::SqliteDriver;
pub use test_helpers::memory_driver_with_seed;
pub use test_helpers::randomized_stress;
pub use test_helpers::{concurrent_readers, concurrent_writers};
pub use test_helpers::{ctx_with_timeout_ms, expired_ctx, memory_driver};
pub use traits::{PersistenceDriver, PersistenceHealth};

use crate::util::config::persistance::{PersistenceBackend, PersistenceConfig};

/// Create a configured persistence driver instance.
///
/// Returns a boxed `PersistenceDriver` that matches the provided config.
/// For `SQLite` this will construct a `SqliteDriver` (feature-gated).
pub async fn make_driver(
    cfg: &PersistenceConfig,
) -> Result<Box<dyn PersistenceDriver>, Box<dyn std::error::Error>> {
    match cfg.backend {
        PersistenceBackend::Memory => Ok(Box::new(InMemoryDriver::new())),
        PersistenceBackend::Sqlite => {
            #[cfg(feature = "persistence")]
            {
                let drv = sqlite::SqliteDriver::from_config(cfg).await?;
                Ok(Box::new(drv))
            }
            #[cfg(not(feature = "persistence"))]
            {
                Err("sqlite support not compiled into this build".into())
            }
        }
        PersistenceBackend::Postgres => {
            #[cfg(feature = "persistence-postgres")]
            {
                let drv = backends::postgres::PostgresDriver::from_config(cfg).await?;
                Ok(Box::new(drv))
            }
            #[cfg(not(feature = "persistence-postgres"))]
            {
                Err("postgres support not compiled into this build".into())
            }
        }
    }
}
