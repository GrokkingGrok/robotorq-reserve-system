//! Minimal Context type for persistence operations.
//!
//! This tiny struct carries two simple things:
//! - Trace headers (like `traceparent`) so database spans can connect
//!   to the bigger story of a request.
//! - An optional deadline that uses a monotonic clock, so timeouts are
//!   reliable even if the system time changes.
//!
//! The `deadline` uses `tokio::time::Instant` (monotonic) and therefore is
//! safe to compare with `tokio` timers. The API stays small on purpose so
//! higher‑level services can create a `Context` from whatever transport
//! they use (HTTP, NATS) without extra dependencies.

use std::collections::HashMap;
use std::time::Duration;

/// Context passed to repository/driver methods.
///
/// # Fields
/// - `trace_headers`: string map of trace headers to attach to spans
/// - `deadline`: optional monotonic deadline (cancellation point)
/// - `metadata`: optional small bag of key/value hints
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// Trace headers (e.g., W3C `traceparent`), transport-agnostic.
    pub trace_headers: HashMap<String, String>,
    /// Monotonic deadline for the operation.
    pub deadline: Option<tokio::time::Instant>,
    /// Optional metadata bag for small key/value hints.
    pub metadata: Option<HashMap<String, String>>,
}

impl Context {
    /// Create an empty context.
    ///
    /// # Returns
    /// - `Context` with no headers or deadline
    ///
    /// # Examples
    /// ```
    /// use commons::util::persistence::Context;
    /// let ctx = Context::new();
    /// assert!(ctx.deadline.is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace_headers: HashMap::new(),
            deadline: None,
            metadata: None,
        }
    }

    /// Create a context with a monotonic deadline `duration` from now.
    ///
    /// # Arguments
    /// - `duration`: how long until the deadline expires
    ///
    /// # Returns
    /// - `Context` with a deadline set `duration` into the future
    ///
    /// # Examples
    /// ```
    /// use commons::util::persistence::Context;
    /// let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(50));
    /// assert!(ctx.deadline.is_some());
    /// ```
    #[must_use]
    pub fn with_deadline_from_now(duration: Duration) -> Self {
        Self {
            trace_headers: HashMap::new(),
            deadline: Some(tokio::time::Instant::now() + duration),
            metadata: None,
        }
    }

    /// Set or replace the deadline to `duration` from now.
    ///
    /// # Arguments
    /// - `duration`: how long from now the new deadline should be
    ///
    /// # Examples
    /// ```
    /// use commons::util::persistence::Context;
    /// let mut ctx = Context::new();
    /// ctx.set_deadline_from_now(std::time::Duration::from_secs(1));
    /// assert!(ctx.deadline.is_some());
    /// ```
    pub fn set_deadline_from_now(&mut self, duration: Duration) {
        self.deadline = Some(tokio::time::Instant::now() + duration);
    }

    /// Clear any previously set deadline.
    ///
    /// # Examples
    /// ```
    /// use commons::util::persistence::Context;
    /// let mut ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(1));
    /// ctx.clear_deadline();
    /// assert!(ctx.deadline.is_none());
    /// ```
    pub fn clear_deadline(&mut self) {
        self.deadline = None;
    }

    /// Returns remaining time until deadline using current monotonic clock.
    /// If the deadline is in the past this returns `Some(Duration::from_millis(0))`.
    ///
    /// # Returns
    /// - `Some(Duration)` if a deadline exists
    /// - `None` if no deadline is set
    #[must_use]
    pub fn time_remaining(&self) -> Option<Duration> {
        self.deadline.map(|dl| {
            let now = tokio::time::Instant::now();
            if dl > now {
                dl - now
            } else {
                Duration::from_millis(0)
            }
        })
    }

    /// Returns remaining time in milliseconds (rounded down) or `None` if no deadline.
    ///
    /// # Returns
    /// - `Some(ms)` when a deadline exists
    /// - `None` when no deadline is set
    #[must_use]
    pub fn time_remaining_ms(&self) -> Option<u128> {
        self.time_remaining().map(|d| d.as_millis())
    }

    /// Returns true when the deadline exists and is already expired.
    ///
    /// # Returns
    /// - `true` when a deadline exists and has passed
    /// - `false` otherwise
    #[must_use]
    pub fn is_expired(&self) -> bool {
        match self.deadline {
            Some(dl) => tokio::time::Instant::now() >= dl,
            None => false,
        }
    }

    /// Helper to insert or replace a trace header.
    ///
    /// # Arguments
    /// - `k`: header name (e.g., `traceparent`)
    /// - `v`: header value
    pub fn insert_trace_header<K: Into<String>, V: Into<String>>(&mut self, k: K, v: V) {
        self.trace_headers.insert(k.into(), v.into());
    }

    /// Helper to fetch a trace header value.
    ///
    /// # Arguments
    /// - `k`: header name
    ///
    /// # Returns
    /// - `Some(&String)` if present
    /// - `None` otherwise
    #[must_use]
    pub fn get_trace_header(&self, k: &str) -> Option<&String> {
        self.trace_headers.get(k)
    }
}
