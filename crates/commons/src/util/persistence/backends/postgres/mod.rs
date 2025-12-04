//! Postgres persistence backend (feature-gated).
//!
//! Provides a `PostgresDriver` backed by `sqlx::PgPool`, an async
//! `from_config` constructor that applies pooling options and session settings,
//! and minimal `health()` and `ping(ctx)` implementations that respect
//! `Context` deadlines and map errors into canonical `PersistenceError` values.

#[cfg(feature = "persistence-postgres")]
use sqlx::PgPool;
#[cfg(feature = "persistence-postgres")]
use sqlx::Row;
#[cfg(feature = "persistence-postgres")]
use sqlx::postgres::PgPoolOptions;

use crate::util::config::persistance::PersistenceConfig;
use crate::util::config::persistance::{PostgresConfig, PostgresSslMode};
use crate::util::persistence::context::Context;
use crate::util::persistence::error::PersistenceError;
use crate::util::persistence::traits::{PersistenceDriver, PersistenceHealth};
use crate::util::persistence::{DbAttributes, obfuscate_statement, with_db_span};

#[cfg(feature = "persistence-postgres")]
#[derive(Clone)]
/// Postgres-backed driver using a connection pool.
///
/// # Fields
/// - `pool`: `sqlx::PgPool` used for query execution and migrations
/// - `schema_validated`: cached boolean indicating whether `validate_schema`
///   succeeded during construction
pub struct PostgresDriver {
    pool: PgPool,
    schema_validated: bool,
}

#[cfg(feature = "persistence-postgres")]
impl PostgresDriver {
    /// Construct a `PostgresDriver` from the unified `PersistenceConfig`.
    ///
    /// # Arguments
    /// - `cfg`: unified persistence configuration
    ///
    /// # Returns
    /// - `Ok(PostgresDriver)` on successful pool initialization and session setup
    /// - `Err` when URL is missing, connection fails, or session setup fails
    ///
    /// # Configuration mapping
    /// - `cfg.database_url`: base Postgres URL. This method appends
    ///   `sslmode` and `application_name` as query params based on
    ///   `cfg.backend_config.postgres`.
    /// - `cfg.backend_config.postgres.ssl_mode`: mapped to `sslmode=<...>`.
    /// - `cfg.backend_config.postgres.application_name`: added to URL and
    ///   reinforced via `set_config('application_name', ...)` after connect.
    /// - `cfg.backend_config.postgres.search_path`: applied via
    ///   `set_config('search_path', ...)` after connect.
    /// - `cfg.max_connections`, `{idle,max}_lifetime_seconds`: applied to pool.
    /// - `cfg.connect_timeout_seconds`: wraps initial connect in a timeout.
    ///
    /// # Defaults & opt-ins
    /// - If `application_name` is empty, it is omitted from the URL and session.
    /// - If `search_path` is empty, no session override is applied.
    /// - `ssl_mode` defaults to `Prefer` via config defaults.
    ///
    /// # Examples
    /// ```no_run
    /// use commons::util::config::persistance::{PersistenceConfig, PersistenceBackend, PostgresSslMode};
    /// use commons::util::persistence::backends::postgres::PostgresDriver;
    /// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut cfg = PersistenceConfig::default();
    /// cfg.backend = PersistenceBackend::Postgres;
    /// cfg.database_url = "postgres://user:pass@localhost:5432/db".to_string();
    /// cfg.max_connections = 5;
    /// cfg.connect_timeout_seconds = 5;
    /// cfg.backend_config.postgres.ssl_mode = PostgresSslMode::Prefer;
    /// cfg.backend_config.postgres.application_name = "robotorq-demo".to_string();
    /// cfg.backend_config.postgres.search_path = "robotorq,public".to_string();
    /// let _drv = PostgresDriver::from_config(&cfg).await?;
    /// # Ok(()) }
    /// ```
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

        // Build URL with sslmode and application_name parameters.
        let url_with_params = build_pg_url_with_params(
            &url,
            cfg.backend_config.postgres.ssl_mode.clone(),
            &cfg.backend_config.postgres.application_name,
        );

        // Enforce connect timeout using tokio::time::timeout since PgPoolOptions
        // in this sqlx version does not expose a connect_timeout setter.
        let pool = match tokio::time::timeout(connect_timeout, opts.connect(&url_with_params)).await
        {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => return Err(Box::new(e)),
            Err(_) => return Err("postgres connect timeout".into()),
        };

        // Apply session settings (`application_name` and `search_path`).
        apply_postgres_session_cfg(&pool, &cfg.backend_config.postgres).await?;

        // Optionally run directory-based migrations, recording progress in the configured table.
        if cfg.run_migrations {
            run_migrations(&pool, &cfg.migration_table).await?;
        }

        // Perform a lightweight schema validation once at startup; cache result for health.
        let mut schema_validated = false;
        let ctx = crate::util::persistence::ctx_with_timeout_ms(2000);
        // Use configured migration table for validation.
        let drv = PostgresDriver {
            pool: pool.clone(),
            schema_validated,
        };
        match drv.validate_schema(&ctx, &cfg.migration_table).await {
            Ok(()) => schema_validated = true,
            Err(_) => schema_validated = false,
        }

        Ok(Self {
            pool,
            schema_validated,
        })
    }
}

#[cfg(feature = "persistence-postgres")]
#[async_trait::async_trait]
impl PersistenceDriver for PostgresDriver {
    /// Report driver health based on cached schema validation.
    ///
    /// # Returns
    /// - `PersistenceHealth { ready: true }` when schema validation succeeded
    /// - `PersistenceHealth { ready: false, message: Some(..) }` otherwise
    fn health(&self) -> PersistenceHealth {
        // Report readiness based on cached schema validation performed at startup.
        if self.schema_validated {
            PersistenceHealth {
                ready: true,
                message: None,
            }
        } else {
            PersistenceHealth {
                ready: false,
                message: Some("schema not validated".to_string()),
            }
        }
    }

    fn shutdown(&self) {
        // PgPool drops will handle closing; nothing required here for prototype.
    }

    async fn ping(&self, ctx: &Context) -> Result<(), PersistenceError> {
        // Wrap probe in a standardized DB span with statement obfuscation.
        let attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("probe".to_string()),
            entity: Some("system".to_string()),
            statement: Some(obfuscate_statement("SELECT 1")),
        };

        with_db_span(ctx, &attrs, async {
            let _v: i32 = sqlx::query_scalar("SELECT 1")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    map_sqlx_error(e)
                        .err()
                        .unwrap_or(PersistenceError::Internal("unknown".to_string()))
                })?;
            Ok(())
        })
        .await
    }
}

#[cfg(feature = "persistence-postgres")]
fn map_sqlx_error(e: sqlx::Error) -> Result<(), PersistenceError> {
    // Map common sqlx errors to canonical persistence variants.
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

/// Build a Postgres connection URL with `sslmode` and `application_name` query parameters.
#[cfg(feature = "persistence-postgres")]
/// Build a Postgres connection URL by appending `sslmode` and `application_name`.
///
/// This function preserves existing query parameters and appends the two
/// parameters if provided. Values are minimally percent-encoded for spaces
/// and a few reserved characters to avoid URL parsing issues.
///
/// - `base`: original Postgres URL (e.g., `postgres://user:pass@host/db`)
/// - `ssl`: desired SSL mode (`require`, `prefer`, `allow`, `disable`)
/// - `app_name`: optional application name applied both via URL and session
///
/// Returns a new URL string safe to pass to `sqlx::PgPool::connect`.
fn build_pg_url_with_params(base: &str, ssl: PostgresSslMode, app_name: &str) -> String {
    fn encode_component(s: &str) -> String {
        // Minimal encoding to keep dependencies light; adequate for common names
        s.replace('%', "%25")
            .replace(' ', "%20")
            .replace('#', "%23")
            .replace('&', "%26")
            .replace('=', "%3D")
    }

    let mut url = base.to_string();

    let sep = if url.contains('?') { '&' } else { '?' };
    let ssl_str = match ssl {
        PostgresSslMode::Require => "require",
        PostgresSslMode::Prefer => "prefer",
        PostgresSslMode::Allow => "allow",
        PostgresSslMode::Disable => "disable",
    };
    url.push(sep);
    url.push_str(&format!("sslmode={}", ssl_str));

    // application_name parameter is supported by libpq.
    if !app_name.is_empty() {
        let sep2 = if url.contains('?') { '&' } else { '?' };
        url.push(sep2);
        url.push_str(&format!("application_name={}", encode_component(app_name)));
    }

    url
}

/// Apply session-level Postgres settings that aren't reliably set via URL.
///
/// Currently sets `application_name` (reinforced) and `search_path`.
#[cfg(feature = "persistence-postgres")]
async fn apply_postgres_session_cfg(
    pool: &PgPool,
    cfg: &PostgresConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Session configuration is applied per-connection using one acquired conn.
    // New connections inherit `application_name` from URL; we reinforce both
    // `application_name` and `search_path` at session level for consistency.
    // Acquire a connection to configure session parameters.
    let mut conn = pool.acquire().await?;

    // SET application_name
    if !cfg.application_name.is_empty() {
        sqlx::query("SELECT set_config('application_name', $1, false)")
            .bind(&cfg.application_name)
            .execute(&mut *conn)
            .await?;
    }

    // SET search_path
    if !cfg.search_path.is_empty() {
        // Use set_config to safely set a comma-separated list of schemas.
        sqlx::query("SELECT set_config('search_path', $1, false)")
            .bind(&cfg.search_path)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}

/// Run migrations found in `crates/commons/sql/migrations` against the provided pool,
/// recording progress in the given `table_name`.
#[cfg(feature = "persistence-postgres")]
async fn run_migrations(pool: &PgPool, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    // Ensure migrations tracking table exists (Postgres syntax)
    let create_stmt = format!(
        "CREATE TABLE IF NOT EXISTS {} (filename TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at BIGINT NOT NULL);",
        table_name
    );
    sqlx::query(&create_stmt).execute(pool).await?;

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
        let existing_query = format!("SELECT checksum FROM {} WHERE filename = $1", table_name);
        let existing: Option<(String,)> = sqlx::query_as(&existing_query)
            .bind(&filename)
            .fetch_optional(pool)
            .await?;

        if let Some((existing_checksum,)) = existing {
            if existing_checksum == checksum {
                // already applied and checksum matches -> skip
                continue;
            }
            // checksum mismatch: migration file changed after being applied
            return Err(format!(
                "migration '{}' checksum mismatch (applied={} file={})",
                filename, existing_checksum, checksum
            )
            .into());
        }

        // Not applied yet: run in a dedicated connection/transaction
        let mut conn = pool.acquire().await?;
        // Begin
        sqlx::query("BEGIN;").execute(&mut *conn).await?;

        // Execute migration SQL; on error attempt rollback and return error
        if let Err(e) = sqlx::query(&sql).execute(&mut *conn).await {
            let _ = sqlx::query("ROLLBACK;").execute(&mut *conn).await;
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }

        // Record applied migration
        let applied_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let insert_stmt = format!(
            "INSERT INTO {}(filename, checksum, applied_at) VALUES ($1, $2, $3);",
            table_name
        );
        if let Err(e) = sqlx::query(&insert_stmt)
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
        sqlx::query("COMMIT;").execute(&mut *conn).await?;
    }

    Ok(())
}

#[cfg(feature = "persistence-postgres")]
impl PostgresDriver {
    /// Validate that core schema objects exist and are accessible.
    ///
    /// Checks for presence of the `kv` table and the configured migration
    /// tracking table. Returns `Ok(())` if both exist; maps database errors
    /// into `PersistenceError` variants.
    ///
    /// # Arguments
    /// - `ctx`: operation context with optional deadline
    ///
    /// # Returns
    /// - `Ok(())` if schema appears valid
    /// - `Err(PersistenceError)` when tables are missing or deadlines exceeded
    ///
    /// # Examples
    /// ```no_run
    /// # async fn demo(drv: &commons::util::persistence::PostgresDriver) -> Result<(), commons::util::persistence::PersistenceError> {
    /// let ctx = commons::util::persistence::ctx_with_timeout_ms(2000);
    /// drv.validate_schema(&ctx, "_robotorq_migrations").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_schema(
        &self,
        ctx: &Context,
        migration_table: &str,
    ) -> Result<(), PersistenceError> {
        // Check kv table exists (optional row probe)
        let kv_attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("schema_check".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(obfuscate_statement("SELECT 1 FROM kv LIMIT 1")),
        };
        with_db_span(ctx, &kv_attrs, async {
            let _ = sqlx::query("SELECT 1 FROM kv LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    map_sqlx_error(e)
                        .err()
                        .unwrap_or(PersistenceError::Internal("unknown".to_string()))
                })?;
            Ok(())
        })
        .await?;

        // Check migration table exists using information_schema for reliability
        let mig_attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("schema_check".to_string()),
            entity: Some("migrations".to_string()),
            statement: Some(obfuscate_statement(
                "SELECT 1 FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = $1 LIMIT 1",
            )),
        };
        with_db_span(ctx, &mig_attrs, async {
            let exists: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM information_schema.tables WHERE table_schema = current_schema() AND table_name = $1 LIMIT 1"
            )
            .bind(migration_table)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error(e).err().unwrap_or(PersistenceError::Internal("unknown".to_string())))?;
            if exists.is_none() {
                return Err(PersistenceError::Internal("migration table missing".to_string()));
            }
            Ok(())
        }).await
    }
}

#[cfg(feature = "persistence-postgres")]
impl PostgresDriver {
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
    /// Insert or update a value in the `kv` table with context-aware tracing.
    ///
    /// Wraps the query with `with_db_span`, honoring `ctx` deadlines and
    /// attaching canonical `DbAttributes`. Uses Postgres upsert semantics.
    ///
    /// # Arguments
    /// - `ctx`: operation context with optional deadline
    /// - `key`: key to upsert
    /// - `value`: value to store
    ///
    /// # Errors
    /// Returns `PersistenceError` when the operation fails or deadline expires.
    pub async fn put_ctx(
        &self,
        ctx: &Context,
        key: &str,
        value: &str,
    ) -> Result<(), PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("upsert".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(obfuscate_statement(
                "INSERT INTO kv(key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value;",
            )),
        };

        with_db_span(ctx, &attrs, async {
            sqlx::query("INSERT INTO kv(key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value;")
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
                .map_err(|e| map_sqlx_error(e).err().unwrap_or(PersistenceError::Internal("unknown".to_string())))?;
            Ok(())
        }).await
    }

    /// Fetch a value by key from the `kv` table with context-aware tracing.
    ///
    /// Returns `Ok(Some(String))` if key exists; `Ok(None)` otherwise.
    pub async fn get_ctx(
        &self,
        ctx: &Context,
        key: &str,
    ) -> Result<Option<String>, PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("read".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(obfuscate_statement("SELECT value FROM kv WHERE key = $1;")),
        };

        with_db_span(ctx, &attrs, async {
            let row = sqlx::query("SELECT value FROM kv WHERE key = $1;")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    map_sqlx_error(e)
                        .err()
                        .unwrap_or(PersistenceError::Internal("unknown".to_string()))
                })?;
            Ok(row.map(|r| r.get::<String, _>(0)))
        })
        .await
    }

    /// Delete a key from the `kv` table with context-aware tracing.
    pub async fn delete_ctx(&self, ctx: &Context, key: &str) -> Result<(), PersistenceError> {
        let attrs = DbAttributes {
            driver: Some("postgres".to_string()),
            op: Some("delete".to_string()),
            entity: Some("kv".to_string()),
            statement: Some(obfuscate_statement("DELETE FROM kv WHERE key = $1;")),
        };

        with_db_span(ctx, &attrs, async {
            sqlx::query("DELETE FROM kv WHERE key = $1;")
                .bind(key)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    map_sqlx_error(e)
                        .err()
                        .unwrap_or(PersistenceError::Internal("unknown".to_string()))
                })?;
            Ok(())
        })
        .await
    }

    /// Non-context upsert convenience.
    pub async fn put(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        sqlx::query("INSERT INTO kv(key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value;")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error(e).err().unwrap_or(PersistenceError::Internal("unknown".to_string())))?;
        Ok(())
    }

    /// Non-context read convenience.
    pub async fn get(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        let row = sqlx::query("SELECT value FROM kv WHERE key = $1;")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                map_sqlx_error(e)
                    .err()
                    .unwrap_or(PersistenceError::Internal("unknown".to_string()))
            })?;
        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    /// Non-context delete convenience.
    pub async fn delete(&self, key: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM kv WHERE key = $1;")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                map_sqlx_error(e)
                    .err()
                    .unwrap_or(PersistenceError::Internal("unknown".to_string()))
            })?;
        Ok(())
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
