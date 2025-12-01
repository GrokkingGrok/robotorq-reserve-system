//! Persistence layer configuration for RoboTorq services.
//!
//! This module defines configuration options for database connections,
//! connection pooling, and storage settings for the unified persistence strategy.

use serde::{Deserialize, Serialize};

/// Persistence layer configuration.
///
/// Configures database connections and storage settings. The RoboTorq system
/// uses a unified persistence strategy across all services to ensure consistency
/// and simplify operational management.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::{PersistenceConfig, PersistenceBackend};
///
/// // Development SQLite configuration
/// let dev_config = PersistenceConfig {
///     backend: PersistenceBackend::Sqlite,
///     database_url: "sqlite://robotorq_dev.db".to_string(),
///     max_connections: 5,
///     min_connections: 1,
///     run_migrations: true,
///     ..Default::default()
/// };
///
/// // Production PostgreSQL configuration
/// let prod_config = PersistenceConfig {
///     backend: PersistenceBackend::Postgres,
///     database_url: "postgresql://user:pass@db.robotorq.internal:5432/robotorq".to_string(),
///     max_connections: 20,
///     min_connections: 5,
///     connect_timeout_seconds: 10,
///     run_migrations: false, // Run migrations separately in production
///     ..Default::default()
/// };
///
/// // In-memory configuration for testing
/// let test_config = PersistenceConfig {
///     backend: PersistenceBackend::Memory,
///     database_url: ":memory:".to_string(),
///     max_connections: 1,
///     run_migrations: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Persistence backend type.
    ///
    /// Determines which database/storage system to use for persistence.
    /// The choice affects operational complexity, performance, and scalability.
    #[serde(default)]
    pub backend: PersistenceBackend,

    /// Database connection string.
    ///
    /// Connection URL or DSN for the database. Format depends on the backend:
    /// - Postgres: "postgresql://user:pass@host:port/database"
    /// - SQLite: "sqlite://path/to/database.db" or ":memory:" for in-memory
    /// - Other: Backend-specific connection string
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// Maximum number of database connections in the pool.
    ///
    /// Controls the connection pool size. Higher values allow more concurrent
    /// database operations but consume more resources.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of database connections to maintain.
    ///
    /// Keeps a baseline number of connections ready to reduce connection latency.
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds.
    ///
    /// Maximum time to wait for a database connection from the pool.
    #[serde(default = "default_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,

    /// Idle connection timeout in seconds.
    ///
    /// How long to keep idle connections before closing them.
    /// Helps manage connection pool size and resource usage.
    #[serde(default = "default_idle_timeout_seconds")]
    pub idle_timeout_seconds: Option<u64>,

    /// Maximum lifetime of a connection in seconds.
    ///
    /// Forces connection renewal to prevent issues with stale connections.
    /// Set to None to disable forced renewal.
    #[serde(default = "default_max_lifetime_seconds")]
    pub max_lifetime_seconds: Option<u64>,

    /// Whether to run database migrations on startup.
    ///
    /// When enabled, services will automatically run pending migrations
    /// during initialization. Should be disabled in production for safety.
    #[serde(default = "default_run_migrations")]
    pub run_migrations: bool,

    /// Migration table name.
    ///
    /// Name of the table used to track applied migrations.
    #[serde(default = "default_migration_table")]
    pub migration_table: String,

    /// Database-specific configuration.
    ///
    /// Backend-specific settings that don't fit common configuration.
    #[serde(default)]
    pub backend_config: BackendSpecificConfig,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            backend: PersistenceBackend::default(),
            database_url: default_database_url(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout_seconds: default_connect_timeout_seconds(),
            idle_timeout_seconds: default_idle_timeout_seconds(),
            max_lifetime_seconds: default_max_lifetime_seconds(),
            run_migrations: default_run_migrations(),
            migration_table: default_migration_table(),
            backend_config: BackendSpecificConfig::default(),
        }
    }
}

/// Supported persistence backends.
///
/// Each backend has different trade-offs in terms of performance, operational
/// complexity, consistency guarantees, and scalability.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::PersistenceBackend;
///
/// // PostgreSQL for production with strong consistency
/// let postgres = PersistenceBackend::Postgres;
///
/// // SQLite for development or small deployments
/// let sqlite = PersistenceBackend::Sqlite;
///
/// // In-memory storage for testing (data lost on restart)
/// let memory = PersistenceBackend::Memory;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceBackend {
    /// PostgreSQL database.
    ///
    /// Strong consistency, ACID transactions, mature tooling.
    /// Good for production deployments requiring strong consistency.
    Postgres,

    /// SQLite database.
    ///
    /// Embedded, file-based storage. Simple operations, good for development
    /// and small-scale deployments. Limited concurrency.
    Sqlite,

    /// In-memory storage.
    ///
    /// No persistence, data lost on restart. Useful for testing and development.
    /// Extremely fast but not suitable for production.
    Memory,
}

impl Default for PersistenceBackend {
    fn default() -> Self {
        PersistenceBackend::Sqlite // Safe default for development
    }
}

/// Backend-specific configuration options.
///
/// Contains settings that are specific to particular database backends
/// and don't fit into the common configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSpecificConfig {
    /// PostgreSQL-specific settings.
    #[serde(default)]
    pub postgres: PostgresConfig,

    /// SQLite-specific settings.
    #[serde(default)]
    pub sqlite: SqliteConfig,
}

impl Default for BackendSpecificConfig {
    fn default() -> Self {
        Self {
            postgres: PostgresConfig::default(),
            sqlite: SqliteConfig::default(),
        }
    }
}

/// PostgreSQL-specific configuration.
///
/// Contains settings specific to PostgreSQL database connections and behavior.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::{PostgresConfig, PostgresSslMode};
///
/// // Production PostgreSQL configuration
/// let prod_config = PostgresConfig {
///     ssl_mode: PostgresSslMode::Require,
///     application_name: "robotorq-gateway".to_string(),
///     search_path: "robotorq,public".to_string(),
/// };
///
/// // Development PostgreSQL configuration
/// let dev_config = PostgresConfig {
///     ssl_mode: PostgresSslMode::Prefer,
///     application_name: "robotorq-dev".to_string(),
///     search_path: "public".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// SSL mode for PostgreSQL connections.
    ///
    /// Controls SSL/TLS requirements for database connections.
    #[serde(default)]
    pub ssl_mode: PostgresSslMode,

    /// Application name sent to PostgreSQL.
    ///
    /// Helps identify connections in database logs and monitoring.
    #[serde(default = "default_postgres_application_name")]
    pub application_name: String,

    /// Schema search path.
    ///
    /// PostgreSQL schemas to search for tables and other objects.
    #[serde(default = "default_postgres_search_path")]
    pub search_path: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            ssl_mode: PostgresSslMode::default(),
            application_name: default_postgres_application_name(),
            search_path: default_postgres_search_path(),
        }
    }
}

/// PostgreSQL SSL modes.
///
/// Controls SSL/TLS requirements for PostgreSQL database connections.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::PostgresSslMode;
///
/// // Require SSL for all connections (production)
/// let require_ssl = PostgresSslMode::Require;
///
/// // Prefer SSL but allow non-SSL (development)
/// let prefer_ssl = PostgresSslMode::Prefer;
///
/// // Allow both SSL and non-SSL connections
/// let allow_both = PostgresSslMode::Allow;
///
/// // Disable SSL (insecure, local development only)
/// let disable_ssl = PostgresSslMode::Disable;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostgresSslMode {
    /// Only allow SSL connections
    Require,
    /// Prefer SSL but allow non-SSL
    Prefer,
    /// Allow both SSL and non-SSL
    Allow,
    /// Disable SSL
    Disable,
}

impl Default for PostgresSslMode {
    fn default() -> Self {
        PostgresSslMode::Prefer
    }
}

/// SQLite-specific configuration.
///
/// Contains settings specific to SQLite database behavior and performance tuning.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::{SqliteConfig, SqliteJournalMode, SqliteSynchronousMode};
///
/// // High-performance SQLite configuration
/// let perf_config = SqliteConfig {
///     foreign_keys: true,
///     journal_mode: SqliteJournalMode::Wal,
///     synchronous: SqliteSynchronousMode::Normal,
///     cache_size_kb: -8192, // 8MB cache
///     busy_timeout_ms: 10000,
/// };
///
/// // Conservative SQLite configuration (safer, slower)
/// let safe_config = SqliteConfig {
///     foreign_keys: true,
///     journal_mode: SqliteJournalMode::Delete,
///     synchronous: SqliteSynchronousMode::Full,
///     cache_size_kb: -1024, // 1MB cache
///     busy_timeout_ms: 30000,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Foreign key enforcement.
    ///
    /// Whether to enforce foreign key constraints in SQLite.
    /// Should generally be enabled for data integrity.
    #[serde(default = "default_sqlite_foreign_keys")]
    pub foreign_keys: bool,

    /// Journal mode.
    ///
    /// Controls how SQLite handles the rollback journal.
    /// WAL mode provides better concurrency.
    #[serde(default)]
    pub journal_mode: SqliteJournalMode,

    /// Synchronous mode.
    ///
    /// Controls how aggressively SQLite syncs to disk.
    /// FULL provides strongest durability guarantees.
    #[serde(default)]
    pub synchronous: SqliteSynchronousMode,

    /// Cache size in kilobytes.
    ///
    /// Memory cache size for SQLite. Negative values mean pages,
    /// positive values mean kilobytes.
    #[serde(default = "default_sqlite_cache_size_kb")]
    pub cache_size_kb: i64,

    /// Busy timeout in milliseconds.
    ///
    /// How long to wait when the database is locked by another connection.
    #[serde(default = "default_sqlite_busy_timeout_ms")]
    pub busy_timeout_ms: u64,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            foreign_keys: default_sqlite_foreign_keys(),
            journal_mode: SqliteJournalMode::default(),
            synchronous: SqliteSynchronousMode::default(),
            cache_size_kb: default_sqlite_cache_size_kb(),
            busy_timeout_ms: default_sqlite_busy_timeout_ms(),
        }
    }
}

/// SQLite journal modes.
///
/// Controls how SQLite handles the rollback journal, affecting concurrency and performance.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::SqliteJournalMode;
///
/// // Write-ahead logging (recommended for concurrent access)
/// let wal = SqliteJournalMode::Wal;
///
/// // Traditional rollback journal
/// let delete = SqliteJournalMode::Delete;
///
/// // In-memory journal (fast but less durable)
/// let memory = SqliteJournalMode::Memory;
///
/// // No journal (fastest but unsafe)
/// let off = SqliteJournalMode::Off;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqliteJournalMode {
    /// Write-ahead logging for better concurrency
    Wal,
    /// Traditional rollback journal
    Delete,
    /// In-memory journal (fast but less safe)
    Memory,
    /// No journal (fastest but unsafe)
    Off,
}

impl Default for SqliteJournalMode {
    fn default() -> Self {
        SqliteJournalMode::Wal
    }
}

/// SQLite synchronous modes.
///
/// Controls how aggressively SQLite syncs data to disk, trading durability for performance.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::persistance::SqliteSynchronousMode;
///
/// // Full synchronization (maximum durability, slowest)
/// let full = SqliteSynchronousMode::Full;
///
/// // Normal synchronization (balanced durability and performance)
/// let normal = SqliteSynchronousMode::Normal;
///
/// // Minimal synchronization (fastest, least durable)
/// let off = SqliteSynchronousMode::Off;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqliteSynchronousMode {
    /// Full synchronization (safest)
    Full,
    /// Synchronize at critical moments
    Normal,
    /// Minimal synchronization (fastest)
    Off,
}

impl Default for SqliteSynchronousMode {
    fn default() -> Self {
        SqliteSynchronousMode::Normal
    }
}

// Default value functions

/// Returns the default database URL.
///
/// Returns `"sqlite://robotorq.db"` as a safe default for development.
/// This creates a local SQLite database file.
fn default_database_url() -> String { "sqlite://robotorq.db".to_string() }

/// Returns the default maximum connection pool size.
///
/// Returns `10` connections as a reasonable default for most applications.
/// Higher values support more concurrent database operations.
fn default_max_connections() -> u32 { 10 }

/// Returns the default minimum connection pool size.
///
/// Returns `1` connection to maintain a baseline connection and reduce
/// connection establishment latency.
fn default_min_connections() -> u32 { 1 }

/// Returns the default connection timeout.
///
/// Returns `30` seconds as a reasonable timeout for establishing
/// new database connections.
fn default_connect_timeout_seconds() -> u64 { 30 }

/// Returns the default idle connection timeout.
///
/// Returns `300` seconds (5 minutes) to close idle connections and
/// manage connection pool size.
fn default_idle_timeout_seconds() -> Option<u64> { Some(300) } // 5 minutes

/// Returns the default maximum connection lifetime.
///
/// Returns `3600` seconds (1 hour) to force connection renewal and
/// prevent issues with stale connections.
fn default_max_lifetime_seconds() -> Option<u64> { Some(3600) } // 1 hour

/// Returns the default migration execution setting.
///
/// Returns `true` to enable automatic migration execution during
/// service startup in development environments.
fn default_run_migrations() -> bool { true }

/// Returns the default migration tracking table name.
///
/// Returns `"_robotorq_migrations"` as the table name for tracking
/// applied database migrations.
fn default_migration_table() -> String { "_robotorq_migrations".to_string() }

/// Returns the default PostgreSQL application name.
///
/// Returns `"robotorq"` to identify connections in database logs
/// and monitoring tools.
fn default_postgres_application_name() -> String { "robotorq".to_string() }

/// Returns the default PostgreSQL schema search path.
///
/// Returns `"public"` as the default schema search path for
/// PostgreSQL database objects.
fn default_postgres_search_path() -> String { "public".to_string() }

/// Returns the default SQLite foreign key enforcement setting.
///
/// Returns `true` to enable foreign key constraints for data integrity.
/// Should generally remain enabled in production.
fn default_sqlite_foreign_keys() -> bool { true }

/// Returns the default SQLite cache size.
///
/// Returns `-2000` (2MB in pages) as a reasonable cache size for
/// most SQLite workloads. Negative values indicate page count.
fn default_sqlite_cache_size_kb() -> i64 { -2000 } // 2MB in pages

/// Returns the default SQLite busy timeout.
///
/// Returns `5000` milliseconds (5 seconds) to wait when the database
/// is locked by another connection before failing the operation.
fn default_sqlite_busy_timeout_ms() -> u64 { 5000 }