//! `SQLite` persistence driver with context-aware, traced operations.
//!
//! Overview
//! - Basic CRUD against a `kv` table with traced, context-aware methods.
//! - Startup bootstrap guarantees `kv(key TEXT PRIMARY KEY, value TEXT NOT NULL)` exists.
//! - Optional schema version validation via a `schema_version(version INTEGER)` table.
//! - Directory-based migrations with checksums and fail-fast semantics.
//!
//! Readiness & Schema Versioning
//! - On initialization (`from_config`), the driver creates `kv` if missing and
//!   performs a lightweight probe.
//! - If a `schema_version(version INTEGER)` table exists, the driver compares its
//!   single row to the expected version constant
//!   (`commons::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION`). A mismatch returns
//!   an error from `from_config`, causing readiness to be not-ready in services.
//! - To repair a mismatch, update the row to the expected version:
//!   `UPDATE schema_version SET version = 1;` (or the current constant) and restart.
//!
//! Migrations
//! - `SQLite` migrations are applied from `crates/commons/sql/migrations` and tracked
//!   in the configured table (default `_robotorq_migrations`). A helper migration also
//!   exists under `sql/sqlite_migrations/0001_create_schema_version.sql` to create and set
//!   the schema version.
//!
//! Feature-gated behind the `persistence` cargo feature; intended for development,
//! CI smoke tests, and small deployments. Postgres is recommended for production workloads.
//!
//! # Examples
//!
//! Create a driver and perform CRUD with a deadline-aware `Context`:
//! ```no_run
//! use commons::util::persistence::sqlite::SqliteDriver;
//! use commons::util::persistence::Context;
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let drv = SqliteDriver::new("sqlite::memory:").await?;
//! let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(100));
//! drv.put_ctx(&ctx, "k", "v").await?;
//! let v = drv.get_ctx(&ctx, "k").await?.unwrap();
//! assert_eq!(v, "v");
//! drv.delete_ctx(&ctx, "k").await?;
//! # Ok(()) }
//! ```

use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::future::Future;

use super::error::PersistenceError;
use super::traits::{PersistenceDriver, PersistenceHealth};
use tracing::instrument;

use super::Context;
use super::span::{DbAttributes, with_db_span};
use crate::util::config::persistance::{
    PersistenceConfig, SqliteConfig, SqliteJournalMode, SqliteSynchronousMode,
};

/// SQLite-backed driver. Connection pool is managed by `sqlx::SqlitePool`.
///
/// # Fields
/// - `pool`: connection pool used for queries and transactions
pub struct SqliteDriver {
    pool: SqlitePool,
}

impl SqliteDriver {
    /// Create a new driver and ensure schema is present.
    ///
    /// This constructor runs directory-based migrations using a default
    /// tracking table `"_robotorq_migrations"`. For production, prefer
    /// `from_config` to control migration execution and table naming.
    ///
    /// # Returns
    /// - `Ok(SqliteDriver)` when the pool is created and migrations applied
    /// - `Err` if the connection fails or migrations cannot be applied
    ///
    /// # Panics
    /// - This function does not panic.
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let drv = SqliteDriver::new("sqlite::memory:").await?;
    /// # Ok(()) }
    /// ```
    pub async fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect(conn_str).await?;

        // By default run migrations when using this simple constructor.
        // Callers that want to control migration execution should use
        // `SqliteDriver::from_config` with a `PersistenceConfig`.
        // Use a conservative default tracking table for PoC constructor.
        run_migrations(&pool, "_robotorq_migrations").await?;

        Ok(Self { pool })
    }

    /// Create a new driver from a `PersistenceConfig`.
    ///
    /// Applies backend-specific PRAGMAs and runs migrations when
    /// `run_migrations = true`, recording progress in `migration_table`.
    /// Use this in services to align driver behavior with config.
    ///
    /// # Returns
    /// - `Ok(SqliteDriver)` when the pool is created and optional migrations applied
    /// - `Err` if connection setup, PRAGMA application, or migrations fail
    ///
    /// # Panics
    /// - This function does not panic.
    ///
    /// # Examples
    /// ```no_run
    /// use commons::util::config::persistance::{PersistenceConfig, PersistenceBackend};
    /// use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let cfg = PersistenceConfig { backend: PersistenceBackend::Sqlite, database_url: "sqlite::memory:".to_string(), run_migrations: true, ..Default::default() };
    /// let drv = SqliteDriver::from_config(&cfg).await?;
    /// # Ok(()) }
    /// ```
    pub async fn from_config(cfg: &PersistenceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect(&cfg.database_url).await?;

        // Apply SQLite backend-specific PRAGMAs from config.
        apply_sqlite_config(&pool, &cfg.backend_config.sqlite).await?;

        if cfg.run_migrations {
            run_migrations(&pool, &cfg.migration_table).await?;
        }

        // Ensure core kv table exists for CRUD tests/integration.
        sqlx::query("CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .execute(&pool)
            .await?;

        // Perform lightweight schema validation: ensure kv and optional schema_version match.
        // Cache-less prototype: health() remains simple; callers can explicitly validate.
        // Check kv
        let _ = sqlx::query("SELECT 1 FROM kv LIMIT 1;")
            .fetch_optional(&pool)
            .await?;
        // Optional schema_version table check — if present, version must match expected.
        use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version' LIMIT 1;",
        )
        .fetch_optional(&pool)
        .await?;
        if exists.is_some() {
            let row: Option<(i32,)> = sqlx::query_as("SELECT version FROM schema_version LIMIT 1;")
                .fetch_optional(&pool)
                .await?;
            if let Some((v,)) = row {
                match u32::try_from(v) {
                    Ok(v_u) if v_u == ROBOTORQ_CONFIG_SCHEMA_VERSION => {}
                    _ => {
                        return Err(format!(
                            "sqlite schema version mismatch: db={v} expected={ROBOTORQ_CONFIG_SCHEMA_VERSION}"
                        )
                        .into());
                    }
                }
            }
        }

        Ok(Self { pool })
    }

    /// Insert or update a value using a traced, context‑aware operation.
    ///
    /// Wraps the query in `with_db_span`, honoring `ctx` deadlines and
    /// attaching canonical `DbAttributes`.
    ///
    /// # Arguments
    /// - `ctx`: operation context containing trace headers and optional deadline
    /// - `key`: primary key for the entry
    /// - `value`: value to store
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(PersistenceError)` on failure or deadline exceeded
    ///
    /// # Panics
    /// - This function does not panic.
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::{Context};
    /// # use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let drv = SqliteDriver::new("sqlite::memory:").await?;
    /// let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(100));
    /// drv.put_ctx(&ctx, "k", "v").await.unwrap();
    /// # Ok(()) }
    /// ```
    pub async fn put_ctx(
        &self,
        ctx: &Context,
        key: &str,
        value: &str,
    ) -> Result<(), PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("sqlite".to_string()),
            op: Some("upsert".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(super::span::obfuscate_statement(
                "INSERT INTO kv(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value;",
            )),
        };

        with_db_span(ctx, &attrs, async {
            sqlx::query("INSERT INTO kv(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value;")
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| PersistenceError::Internal(e.to_string()))?;
            Ok(())
        }).await
    }

    /// Fetch a value using a traced, context‑aware operation.
    ///
    /// # Arguments
    /// - `ctx`: operation context
    /// - `key`: primary key for the entry
    ///
    /// # Returns
    /// - `Ok(Some(String))` when the key exists
    /// - `Ok(None)` if no row exists
    /// - `Err(PersistenceError)` on error or deadline exceeded
    ///
    /// # Panics
    /// - This function does not panic.
    pub async fn get_ctx(
        &self,
        ctx: &Context,
        key: &str,
    ) -> Result<Option<String>, PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("sqlite".to_string()),
            op: Some("read".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(super::span::obfuscate_statement(
                "SELECT value FROM kv WHERE key = ?;",
            )),
        };

        with_db_span(ctx, &attrs, async {
            let row = sqlx::query("SELECT value FROM kv WHERE key = ?;")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| PersistenceError::Internal(e.to_string()))?;
            Ok(row.map(|r| r.get::<String, _>(0)))
        })
        .await
    }

    /// Delete a key using a traced, context‑aware operation.
    ///
    /// # Arguments
    /// - `ctx`: operation context
    /// - `key`: primary key for the entry
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(PersistenceError)` on error or deadline exceeded
    ///
    /// # Panics
    /// - This function does not panic.
    pub async fn delete_ctx(&self, ctx: &Context, key: &str) -> Result<(), PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("sqlite".to_string()),
            op: Some("delete".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(super::span::obfuscate_statement(
                "DELETE FROM kv WHERE key = ?;",
            )),
        };

        with_db_span(ctx, &attrs, async {
            sqlx::query("DELETE FROM kv WHERE key = ?;")
                .bind(key)
                .execute(&self.pool)
                .await
                .map_err(|e| PersistenceError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }
    /// Put a value into the `kv` table.
    ///
    /// # Arguments
    /// - `key`: primary key
    /// - `value`: value to store
    ///
    /// # Returns
    /// - `Ok(())` on success; `Err(PersistenceError)` on failure
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let drv = SqliteDriver::new("sqlite::memory:").await?;
    /// drv.put("k","v").await.unwrap();
    /// # Ok(()) }
    /// ```
    pub async fn put(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        sqlx::query("INSERT INTO kv(key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value;")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Get a value from the `kv` table.
    ///
    /// # Arguments
    /// - `key`: primary key
    ///
    /// # Returns
    /// - `Ok(Some(String))` when present; `Ok(None)` when missing; `Err` on failure
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let drv = SqliteDriver::new("sqlite::memory:").await?;
    /// drv.put("k","v").await.unwrap();
    /// let v = drv.get("k").await.unwrap();
    /// assert_eq!(v, Some("v".to_string()));
    /// # Ok(()) }
    /// ```
    pub async fn get(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        let row = sqlx::query("SELECT value FROM kv WHERE key = ?;")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PersistenceError::Internal(e.to_string()))?;

        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    /// Delete a key from the `kv` table.
    ///
    /// # Arguments
    /// - `key`: primary key
    ///
    /// # Returns
    /// - `Ok(())` on success; `Err(PersistenceError)` on failure
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::sqlite::SqliteDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let drv = SqliteDriver::new("sqlite::memory:").await?;
    /// drv.put("k","v").await.unwrap();
    /// drv.delete("k").await.unwrap();
    /// # Ok(()) }
    /// ```
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
    ///
    /// # Returns
    /// - `Ok(())` if the database responds
    /// - `Err(PersistenceError::Unavailable)` if the query fails
    ///
    /// # Panics
    /// - This function does not panic.
    #[instrument(level = "debug", skip(self))]
    pub async fn ping(&self) -> Result<(), PersistenceError> {
        let _ = sqlx::query("SELECT 1;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PersistenceError::Unavailable(e.to_string()))?;
        Ok(())
    }

    /// Run a closure inside a SQL transaction.
    ///
    /// The closure receives a mutable connection to run queries. If the
    /// closure returns `Ok`, the transaction is committed; if it returns `Err`,
    /// the transaction is rolled back and the error is propagated.
    ///
    /// Note: This `PoC` uses manual BEGIN/COMMIT/ROLLBACK for simplicity.
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

/// Run migrations found in `crates/commons/sql/migrations` into the given pool,
/// recording progress in the provided `table_name`.
#[allow(clippy::cast_possible_wrap)]
async fn run_migrations(
    pool: &SqlitePool,
    table_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = run_migrations_structured(pool, table_name).await?;
    Ok(())
}

/// Structured migration outcome for observability and tests.
#[derive(Debug, Default, Clone)]
pub struct MigrationOutcome {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub dirty: Option<String>,
}

/// Structured migration runner (idempotent, fail-fast) for `SQLite`.
pub async fn run_migrations_structured(
    pool: &SqlitePool,
    table_name: &str,
) -> Result<MigrationOutcome, Box<dyn std::error::Error>> {
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
        return Ok(MigrationOutcome::default());
    }

    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "sql"))
        .collect();

    // Sort lexicographically so files like 0001_... run before 0002_...
    entries.sort_by_key(std::fs::DirEntry::path);

    // Ensure migrations tracking table exists
    let create_stmt = format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (filename TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at INTEGER NOT NULL);"
    );
    sqlx::query(&create_stmt)
        .execute(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let mut outcome = MigrationOutcome::default();
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
        let existing_query = format!("SELECT checksum FROM {table_name} WHERE filename = ?");
        let existing: Option<(String,)> = sqlx::query_as(&existing_query)
            .bind(&filename)
            .fetch_optional(pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        if let Some((existing_checksum,)) = existing {
            if existing_checksum == checksum {
                // already applied and checksum matches -> skip
                outcome.skipped.push(filename.clone());
                continue;
            }
            // checksum mismatch: migration file changed after being applied
            outcome.dirty = Some(format!(
                "migration '{filename}' checksum mismatch (applied={existing_checksum} file={checksum})"
            ));
            return Err(outcome.dirty.clone().unwrap().into());
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
            outcome.dirty = Some(format!("apply failed for {filename}: {e}"));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }

        // Record applied migration (safe conversion from u64 -> i64)
        let applied_at = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
            .unwrap_or(i64::MAX);
        let insert_stmt =
            format!("INSERT INTO {table_name}(filename, checksum, applied_at) VALUES (?, ?, ?);");
        if let Err(e) = sqlx::query(&insert_stmt)
            .bind(&filename)
            .bind(&checksum)
            .bind(applied_at)
            .execute(&mut *conn)
            .await
        {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            outcome.dirty = Some(format!("record failed for {filename}: {e}"));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }

        // Commit
        sqlx::query("COMMIT;")
            .execute(&mut *conn)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        outcome.applied.push(filename);
    }
    Ok(outcome)
}

/// Public wrapper to run migrations against a `SQLite` connection string, with a custom table.
///
/// This is provided for lightweight tooling that wants to invoke the
/// migration runner without constructing a full `SqliteDriver`.
pub async fn run_migrations_with_conn_str(
    conn_str: &str,
    table_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect(conn_str).await?;
    run_migrations(&pool, table_name).await?;
    Ok(())
}

/// Apply `SQLite` PRAGMAs according to the provided `SqliteConfig`.
async fn apply_sqlite_config(
    pool: &SqlitePool,
    cfg: &SqliteConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::Executor;
    let mut conn = pool.acquire().await?;

    // Foreign keys
    let fk_val = i32::from(cfg.foreign_keys);
    conn.execute(sqlx::query(&format!("PRAGMA foreign_keys = {fk_val};")))
        .await?;

    // Journal mode
    let journal_mode = match cfg.journal_mode {
        SqliteJournalMode::Wal => "WAL",
        SqliteJournalMode::Delete => "DELETE",
        SqliteJournalMode::Memory => "MEMORY",
        SqliteJournalMode::Off => "OFF",
    };
    // journal_mode returns a row; using execute is fine for side-effect
    conn.execute(sqlx::query(&format!(
        "PRAGMA journal_mode = {journal_mode};"
    )))
    .await?;

    // Synchronous
    let synchronous = match cfg.synchronous {
        SqliteSynchronousMode::Full => "FULL",
        SqliteSynchronousMode::Normal => "NORMAL",
        SqliteSynchronousMode::Off => "OFF",
    };
    conn.execute(sqlx::query(&format!("PRAGMA synchronous = {synchronous};")))
        .await?;

    // Cache size (negative indicates pages)
    conn.execute(sqlx::query(&format!(
        "PRAGMA cache_size = {};",
        cfg.cache_size_kb
    )))
    .await?;

    // Busy timeout
    conn.execute(sqlx::query(&format!(
        "PRAGMA busy_timeout = {};",
        cfg.busy_timeout_ms
    )))
    .await?;

    Ok(())
}
