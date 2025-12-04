use thiserror::Error;

/// Persistence-related errors (database pools, queries, migrations).
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// Errors allocating or interacting with a connection pool.
    #[error("PoolError: {0}")]
    Pool(String),

    /// Errors executing queries or mapping results.
    #[error("QueryError: {0}")]
    Query(String),

    /// Migration-related failures.
    #[error("MigrationError: {0}")]
    Migration(String),

    /// Other persistence errors.
    #[error("Unknown persistence error: {0}")]
    Other(String),
}

#[cfg(feature = "persistence")]
impl From<sqlx::Error> for PersistenceError {
    fn from(e: sqlx::Error) -> Self {
        PersistenceError::Query(format!("sqlx error: {e}"))
    }
}
