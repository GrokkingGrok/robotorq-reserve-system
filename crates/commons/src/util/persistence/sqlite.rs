//! Lightweight `SQLite` persistence driver (`PoC`)
//!
//! This file provides a small proof-of-concept `SqliteDriver` that implements
//! basic CRUD operations for a simple `kv` table and implements the
//! `PersistenceDriver` trait for health/shutdown. It's feature-gated behind
//! the `persistence` feature and intentionally minimal — it's intended for
//! functional verification and quick integration tests, not as a production
//! Postgres-ready implementation.

use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::future::Future;

use super::error::PersistenceError;
use super::traits::{PersistenceDriver, PersistenceHealth};
use tracing::instrument;

/// SQLite-backed driver. Connection pool is managed by `sqlx::SqlitePool`.
pub struct SqliteDriver {
    pool: SqlitePool,
}

impl SqliteDriver {
    /// Create a new driver and ensure schema is present.
    pub async fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect(conn_str).await?;

        // By default run migrations when using this simple constructor.
        // Callers that want to control migration execution should use
        // `SqliteDriver::from_config` with a `PersistenceConfig`.
        run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// Create a new driver from a `PersistenceConfig`.
    ///
    /// This respects `PersistenceConfig::run_migrations` so callers can
    /// opt-out of running migrations at startup by setting that flag to
    /// `false` (recommended for production where migrations run separately).
    pub async fn from_config(
        cfg: &crate::util::config::persistance::PersistenceConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect(&cfg.database_url).await?;

        if cfg.run_migrations {
            run_migrations(&pool).await?;
        }

        Ok(Self { pool })
    }
    /// Put a value into the kv table.
    pub async fn put(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        sqlx::query("INSERT INTO kv(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value;")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Get a value from the kv table.
    pub async fn get(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        let row = sqlx::query("SELECT value FROM kv WHERE key = ?;")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;

        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    /// Delete a key from the kv table.
    pub async fn delete(&self, key: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM kv WHERE key = ?;")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;
        Ok(())
    }
}

impl PersistenceDriver for SqliteDriver {
    fn health(&self) -> PersistenceHealth {
        // Minimal health check: pool exists. Deeper checks can run a lightweight query.
        PersistenceHealth {
            ready: true,
            message: Some("sqlite pool initialized".to_string()),
        }
    }

    fn shutdown(&self) {
        // Dropping the pool will close connections.
        // No-op here; pool will be dropped when driver is dropped.
    }
}

impl SqliteDriver {
    /// Lightweight ping: run a `SELECT 1` to verify DB responsiveness.
    #[instrument(level = "debug", skip(self))]
    pub async fn ping(&self) -> Result<(), PersistenceError> {
        let _ = sqlx::query("SELECT 1;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PersistenceError::Unavailable(e.to_string()))?;
        Ok(())
    }

    /// Run a closure inside a SQL transaction. The closure receives a mutable
    /// reference to the `sqlx::Transaction` to run queries. If the closure
    /// returns `Ok`, the transaction is committed; if it returns `Err`, the
    /// transaction is rolled back and the error is propagated.
    pub async fn run_transaction<F, Fut, T>(&self, f: F) -> Result<T, PersistenceError>
    where
        for<'c> F: FnOnce(&'c mut sqlx::SqliteConnection) -> Fut,
        for<'c> Fut: Future<Output = Result<T, PersistenceError>> + 'c,
    {
        // Acquire a connection from the pool and run manual BEGIN/COMMIT/ROLLBACK
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| PersistenceError::Unavailable(e.to_string()))?;

        sqlx::query("BEGIN;")
            .execute(&mut *conn)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;

        let res = f(&mut conn).await;
        match res {
            Ok(v) => {
                sqlx::query("COMMIT;")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| PersistenceError::Internal(e.to_string()))?;
                Ok(v)
            }
            Err(err) => {
                let _ = sqlx::query("ROLLBACK;")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| PersistenceError::Internal(e.to_string()));
                Err(err)
            }
        }
    }

    /// Acquire a pooled connection from the driver. Useful when callers need
    /// direct control over transactions (BEGIN/COMMIT/ROLLBACK) or multiple
    /// statements on the same connection.
    pub async fn acquire_connection(
        &self,
    ) -> Result<sqlx::pool::PoolConnection<sqlx::Sqlite>, PersistenceError> {
        self.pool
            .acquire()
            .await
            .map_err(|e| PersistenceError::Unavailable(e.to_string()))
    }
}

#[allow(clippy::cast_possible_wrap)]
async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    use blake3;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Locate migrations directory relative to the crate manifest.
    let mut migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    migrations_dir.push("sql");
    migrations_dir.push("migrations");

    if !migrations_dir.exists() {
        // Nothing to run
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "sql"))
        .collect();

    // Sort lexicographically so files like 0001_... run before 0002_...
    entries.sort_by_key(std::fs::DirEntry::path);

    // Ensure migrations tracking table exists
    sqlx::query("CREATE TABLE IF NOT EXISTS schema_migrations (filename TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at INTEGER NOT NULL);")
        .execute(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    for ent in entries {
        let path = ent.path();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid migration filename")?
            .to_string();

        let sql = fs::read_to_string(&path)?;
        let checksum = blake3::hash(sql.as_bytes()).to_hex().to_string();

        // Check if this migration was already applied
        let existing: Option<(String,)> =
            sqlx::query_as("SELECT checksum FROM schema_migrations WHERE filename = ?")
                .bind(&filename)
                .fetch_optional(pool)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        if let Some((existing_checksum,)) = existing {
            if existing_checksum == checksum {
                // already applied and checksum matches -> skip
                continue;
            }
            // checksum mismatch: migration file changed after being applied
            return Err(format!(
                "migration '{filename}' checksum mismatch (applied={existing_checksum} file={checksum})"
            )
            .into());
        }

        // Not applied yet: run in a dedicated connection/transaction
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        // Begin
        sqlx::query("BEGIN;")
            .execute(&mut *conn)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // Execute migration SQL; on error attempt rollback and return error
        if let Err(e) = sqlx::query(&sql).execute(&mut *conn).await {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }

        // Record applied migration
        let applied_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        if let Err(e) = sqlx::query(
            "INSERT INTO schema_migrations(filename, checksum, applied_at) VALUES (?, ?, ?);",
        )
        .bind(&filename)
        .bind(&checksum)
        .bind(applied_at)
        .execute(&mut *conn)
        .await
        {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }

        // Commit
        sqlx::query("COMMIT;")
            .execute(&mut *conn)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    }

    Ok(())
}

/// Public wrapper to run migrations against a `SQLite` connection string.
///
/// This is provided for lightweight tooling that wants to invoke the
/// migration runner without constructing a full `SqliteDriver`.
pub async fn run_migrations_with_conn_str(
    conn_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect(conn_str).await?;
    run_migrations(&pool).await?;
    Ok(())
}
