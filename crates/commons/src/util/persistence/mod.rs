//! Persistence primitives and drivers.
//!
//! Provides transport-agnostic building blocks for DB access:
//! - `Context`: trace headers + optional monotonic deadline
//! - `PersistenceError`: canonical error taxonomy (Unavailable/Timeout/etc.)
//! - `DbAttributes`: standardized span attributes for DB operations
//! - `with_db_span`: wrapper to run an async op with tracing + timeout enforcement
//! - `PersistenceDriver`: trait for health/readiness integration (cached and strict)
//! - Drivers: `InMemoryDriver`, `SqliteDriver`, `PostgresDriver` (feature-gated)
//!
//! # Quick Start
//! ```rust
//! use commons::util::persistence::{Context, DbAttributes, PersistenceError, with_db_span};
//! # async fn example(ctx: &Context) -> Result<(), PersistenceError> {
//! let attrs = DbAttributes {
//!     driver: Some("sqlite".to_string()),
//!     op: Some("select".to_string()),
//!     entity: Some("system".to_string()),
//!     statement: Some("select 1".to_string()),
//! };
//! let res: Result<(), PersistenceError> = with_db_span(ctx, &attrs, async { Ok::<_, PersistenceError>(()) }).await;
//! assert!(res.is_ok());
//! Ok(())
//! # }
//! ```
//!
//! # Notes
//! - Use `Context::with_deadline_from_now` to enforce per-op timeouts.
//! - Obfuscate statements via `obfuscate_statement` to avoid leaking PII.
//! - Prefer `health_now()` for strict readiness; `health()` is cached.
//!
//! See also: `docs/PERSISTENCE_SQLITE.md` for `SQLite` defaults, readiness, and schema versioning.

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
pub use span::{DbAttributes, obfuscate_statement, with_db_span};
#[cfg(feature = "persistence")]
pub use sqlite::SqliteDriver;
pub use test_helpers::memory_driver_with_seed;
pub use test_helpers::randomized_stress;
pub use test_helpers::{concurrent_readers, concurrent_writers};
pub use test_helpers::{ctx_with_timeout_ms, expired_ctx, memory_driver};
pub use traits::{PersistenceDriver, PersistenceHealth};

use crate::util::config::persistance::{PersistenceBackend, PersistenceConfig};

/// Create a persistence driver from `PersistenceConfig`.
///
/// Picks the concrete backend based on `cfg.backend`, initializes it using
/// the provided settings (URL/DSN, pool sizes, timeouts, etc.), and returns a
/// boxed [`PersistenceDriver`].
///
/// Supported backends:
/// - `Memory` (always available): in‑process, ephemeral store for tests/dev.
/// - `SQLite` (requires `persistence` feature): file or `:memory:` via `SqliteDriver`.
/// - `Postgres` (requires `persistence-postgres` feature): DSN‑based via `PostgresDriver`.
///
/// Returns
/// - `Ok(Box<dyn PersistenceDriver>)` when the requested backend is available and
///   successfully initialized.
/// - `Err(_)` if the backend is not compiled into this build or initialization fails
///   (e.g., invalid URL, connection refused, misconfiguration).
///
/// Panics
/// - This function does not panic.
///
/// Examples
/// Create an in‑memory driver for tests or local experiments:
/// ```no_run
/// use commons::util::persistence::{make_driver, PersistenceDriver, PersistenceHealth};
/// use commons::util::config::persistance::{PersistenceConfig, PersistenceBackend};
///
/// // Start from defaults and switch to an in‑memory backend
/// let mut cfg = PersistenceConfig::default();
/// cfg.backend = PersistenceBackend::Memory;
///
/// // Create the driver on a Tokio runtime
/// let rt = tokio::runtime::Runtime::new().unwrap();
/// let driver = rt.block_on(make_driver(&cfg)).expect("driver created");
///
/// // Strict (per‑request) health check
/// let h = rt.block_on(driver.health_now());
/// assert!(h.ready);
/// ```
///
/// Notes
/// - For `SQLite`/`Postgres`, set `database_url` and pool limits in `PersistenceConfig`.
/// - In production, consider running migrations out‑of‑band (`run_migrations = false`).
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
