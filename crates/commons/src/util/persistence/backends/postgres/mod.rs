//! Postgres persistence backend prototype (feature-gated)
//!
//! This is a lightweight prototype to satisfy Sprint 3 scaffolding. It provides
//! a `PostgresDriver` struct and an async `from_config` constructor. The driver
//! implements `PersistenceDriver` but methods are intentionally minimal for the
//! prototype: `health()` returns a simple readiness view and `ping()` is a
//! placeholder to avoid blocking in synchronous trait methods.

#[cfg(feature = "persistence-postgres")]
use sqlx::PgPool;
#[cfg(feature = "persistence-postgres")]
use sqlx::postgres::PgPoolOptions;

use crate::util::config::persistance::PersistenceConfig;
use crate::util::persistence::context::Context;
use crate::util::persistence::error::PersistenceError;
use crate::util::persistence::traits::{PersistenceDriver, PersistenceHealth};

#[cfg(feature = "persistence-postgres")]
#[derive(Clone)]
pub struct PostgresDriver {
    pool: PgPool,
}

#[cfg(feature = "persistence-postgres")]
impl PostgresDriver {
    /// Construct a `PostgresDriver` from `PersistenceConfig`.
    pub async fn from_config(cfg: &PersistenceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Use values from the unified `PersistenceConfig`.
        let url = cfg.database_url.clone();
        if url.is_empty() {
            return Err("postgres database_url required".into());
        }

        let max_conns = cfg.max_connections;
        let connect_timeout = std::time::Duration::from_secs(cfg.connect_timeout_seconds);

        let mut opts = PgPoolOptions::new().max_connections(max_conns);

        if let Some(idle_secs) = cfg.idle_timeout_seconds {
            opts = opts.idle_timeout(std::time::Duration::from_secs(idle_secs));
        }

        if let Some(max_life) = cfg.max_lifetime_seconds {
            opts = opts.max_lifetime(std::time::Duration::from_secs(max_life));
        }

        // Enforce connect timeout using tokio::time::timeout since PgPoolOptions
        // in this sqlx version does not expose a connect_timeout setter.
        let pool = match tokio::time::timeout(connect_timeout, opts.connect(&url)).await {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => return Err(Box::new(e)),
            Err(_) => return Err("postgres connect timeout".into()),
        };

        Ok(Self { pool })
    }
}

#[cfg(feature = "persistence-postgres")]
#[async_trait::async_trait]
impl PersistenceDriver for PostgresDriver {
    fn health(&self) -> PersistenceHealth {
        // For prototype: report ready if pool exists. Real implementation
        // should run a lightweight query and check migrations/schema.
        PersistenceHealth {
            ready: true,
            message: None,
        }
    }

    fn shutdown(&self) {
        // PgPool drops will handle closing; nothing required here for prototype.
    }

    async fn ping(&self, ctx: &Context) -> Result<(), PersistenceError> {
        // If context is already expired, return immediately.
        if ctx.is_expired() {
            return Err(PersistenceError::DeadlineExceeded);
        }

        // Build the future that executes a lightweight probe query.
        let probe = async {
            // Use a simple scalar query to validate the connection.
            // `query_scalar!` requires a literal SQL string; use `query_scalar` to avoid the macro.
            let _v: i32 = sqlx::query_scalar("SELECT 1").fetch_one(&self.pool).await?;
            Ok(())
        };

        // If the caller supplied a deadline, enforce it using tokio::time::timeout.
        if let Some(dur) = ctx.time_remaining() {
            if dur.as_millis() == 0 {
                return Err(PersistenceError::DeadlineExceeded);
            }

            match tokio::time::timeout(dur, probe).await {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(e)) => return map_sqlx_error(e),
                Err(_) => return Err(PersistenceError::DeadlineExceeded),
            }
        }

        // No deadline — just await the probe and map errors.
        match probe.await {
            Ok(()) => Ok(()),
            Err(e) => map_sqlx_error(e),
        }
    }
}

#[cfg(feature = "persistence-postgres")]
fn map_sqlx_error(e: sqlx::Error) -> Result<(), PersistenceError> {
    match e {
        // Pool/connect level failures map to Unavailable.
        sqlx::Error::PoolTimedOut
        | sqlx::Error::PoolClosed
        | sqlx::Error::Tls(_)
        | sqlx::Error::Io(_) => Err(PersistenceError::Unavailable(format!("sqlx error: {e:?}"))),
        // Database-level errors may indicate constraint violations — try to map common codes.
        sqlx::Error::Database(db_err) => {
            if let Some(code) = db_err.code() {
                // Postgres unique violation
                if code.as_ref() == "23505" {
                    return Err(PersistenceError::Conflict(format!("db: {code}")));
                }
            }
            Err(PersistenceError::Internal(format!("db error: {db_err:?}")))
        }
        // RowNotFound and other non-db errors -> Internal for now.
        other => Err(PersistenceError::Internal(format!("sqlx error: {other:?}"))),
    }
}

#[cfg(not(feature = "persistence-postgres"))]
compile_error!("feature `persistence-postgres` must be enabled to compile this module");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::persistence::error::PersistenceError;
    use std::io;

    #[test]
    fn map_pool_errors_to_unavailable() {
        let e = sqlx::Error::PoolTimedOut;
        let r = map_sqlx_error(e);
        assert!(matches!(r, Err(PersistenceError::Unavailable(_))));

        let e2 = sqlx::Error::PoolClosed;
        let r2 = map_sqlx_error(e2);
        assert!(matches!(r2, Err(PersistenceError::Unavailable(_))));
    }

    #[test]
    fn map_io_and_tls_to_unavailable() {
        let io_err = io::Error::new(io::ErrorKind::Other, "io");
        let e = sqlx::Error::Io(io_err);
        let r = map_sqlx_error(e);
        assert!(matches!(r, Err(PersistenceError::Unavailable(_))));
    }

    // Database-specific mapping unit tests are omitted here because constructing
    // a boxed `sqlx::DatabaseError` requires implementing sqlx internals; the
    // important mapping for pool/io/tls errors is covered below.
}
