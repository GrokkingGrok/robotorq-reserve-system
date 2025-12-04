//! Database span helpers and attributes.
//!
//! Provides a standard attribute bag and `with_db_span` wrapper to
//! consistently annotate persistence operations.

use std::future::Future;
use std::time::Duration;
use tracing::Instrument;

use super::context::Context;
use super::error::PersistenceError;

/// Canonical attributes for DB spans.
#[derive(Clone, Debug, Default)]
pub struct DbAttributes {
    /// Driver name (e.g., memory, sqlite, postgres).
    pub driver: Option<String>,
    /// Logical operation (e.g., read, write, upsert).
    pub op: Option<String>,
    /// Target entity/table name.
    pub entity: Option<String>,
    /// Obfuscated query or statement id.
    pub statement: Option<String>,
}

/// Wrap an async persistence operation with a standardized DB span and
/// respect the optional monotonic deadline in `ctx`.
///
/// The provided future must return `Result<T, PersistenceError>` so timeouts
/// can be mapped to `PersistenceError::DeadlineExceeded`.
pub async fn with_db_span<Fut, T>(
    ctx: &Context,
    attrs: &DbAttributes,
    fut: Fut,
) -> Result<T, PersistenceError>
where
    Fut: Future<Output = Result<T, PersistenceError>>,
{
    // Compute canonical string values for fields (empty string when absent)
    let driver_str = attrs.driver.as_deref().unwrap_or("");
    let op_str = attrs.op.as_deref().unwrap_or("");
    let entity_str = attrs.entity.as_deref().unwrap_or("");
    let stmt_str = attrs.statement.as_deref().unwrap_or("");
    let timeout_ms: u128 = ctx.time_remaining().map_or(0, |d| d.as_millis());

    let span = tracing::info_span!(
        "db.op",
        db.driver = %driver_str,
        db.operation = %op_str,
        db.entity = %entity_str,
        db.statement = %stmt_str,
        db.timeout_ms = %timeout_ms
    );

    // If a deadline exists, apply a tokio timeout to the operation and map
    // a timeout error to `PersistenceError::DeadlineExceeded`.
    if let Some(remaining) = ctx.time_remaining() {
        let dur: Duration = remaining;
        match tokio::time::timeout(dur, fut.instrument(span)).await {
            Ok(res) => res,
            Err(_) => Err(PersistenceError::DeadlineExceeded),
        }
    } else {
        fut.instrument(span).await
    }
}
