//! Database span helpers and attributes.
//!
//! This module helps every database operation create a clear, consistent
//! tracing span with a few standard fields (driver, operation, entity,
//! statement id, timeout). The goal is to make it easy to understand
//! "what DB thing just happened" when looking at logs or traces.

use std::future::Future;
use std::time::Duration;
use tracing::Instrument;

use super::context::Context;
use super::error::PersistenceError;

/// Canonical attributes for DB spans.
///
/// # Fields
/// - `driver`: which backend is used (e.g., `memory`, `sqlite`, `postgres`)
/// - `op`: operation name (e.g., `read`, `write`, `upsert`)
/// - `entity`: target entity or table (e.g., `kv`)
/// - `statement`: obfuscated query identifier (see `obfuscate_statement`)
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

/// Obfuscate a raw SQL statement into a stable identifier suitable for logs.
///
/// Returns a `stmt:<hex-digest>` string derived from a BLAKE3 hash of the
/// input. This avoids leaking sensitive literals or schema details while
/// allowing correlation across identical statements.
///
/// # Arguments
/// - `raw`: the original SQL text to obfuscate
///
/// # Returns
/// - A stable identifier string of the form `stmt:<hex>` (length 69)
///
/// # Examples
/// ```
/// use commons::util::persistence::obfuscate_statement;
/// let id = obfuscate_statement("SELECT * FROM kv WHERE key = $1");
/// assert!(id.starts_with("stmt:"));
/// assert_eq!(id.len(), 5 + 64); // "stmt:" (5) + 64 hex chars
/// ```
#[must_use]
pub fn obfuscate_statement(raw: &str) -> String {
    let digest = blake3::hash(raw.as_bytes()).to_hex().to_string();
    format!("stmt:{digest}")
}

/// Wrap an async persistence operation with a standardized DB span.
///
/// This helper starts a `db.op` span with fields from `attrs`, applies
/// a timeout if `ctx` has a deadline, and maps timeout errors to
/// `PersistenceError::DeadlineExceeded`.
///
/// # Arguments
/// - `ctx`: operation context with optional monotonic deadline
/// - `attrs`: canonical DB attributes to attach to the span
/// - `fut`: future that runs the actual DB operation
///
/// # Returns
/// - `Ok(T)` when the operation succeeds
/// - `Err(PersistenceError::DeadlineExceeded)` if the deadline expires
/// - `Err(PersistenceError)` when the operation fails
///
/// # Examples
/// ```no_run
/// use commons::util::persistence::{Context, PersistenceError};
/// use commons::util::persistence::{DbAttributes, with_db_span};
/// # async fn demo() -> Result<(), PersistenceError> {
/// let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(50));
/// let attrs = DbAttributes { driver: Some("sqlite".into()), op: Some("read".into()), entity: Some("kv".into()), statement: None };
/// let val: Result<i32, PersistenceError> = with_db_span(&ctx, &attrs, async { Ok(42) }).await;
/// assert_eq!(val.unwrap(), 42);
/// # Ok(()) }
/// ```
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
        // If no time remains, fail fast without executing the operation.
        if remaining.is_zero() {
            return Err(PersistenceError::DeadlineExceeded);
        }

        let dur: Duration = remaining;
        match tokio::time::timeout(dur, fut.instrument(span)).await {
            Ok(res) => res,
            Err(_) => Err(PersistenceError::DeadlineExceeded),
        }
    } else {
        fut.instrument(span).await
    }
}
