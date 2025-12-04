//! Minimal Context type for persistence operations.
//!
//! Holds transport-agnostic trace headers and an optional monotonic deadline
//! used by persistence drivers to reason about remaining timeouts.
//!
//! The `deadline` uses `tokio::time::Instant` (monotonic) and therefore is
//! safe to compare with `tokio` timers. This module keeps the API small and
//! intentionally avoids coupling to global time providers; higher-level
//! services can construct `Context` values from request-scoped deadlines.

use std::collections::HashMap;
use std::time::Duration;

/// Context passed to repository/driver methods.
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            trace_headers: HashMap::new(),
            deadline: None,
            metadata: None,
        }
    }

    /// Create a context with a monotonic deadline `duration` from now.
    #[must_use]
    pub fn with_deadline_from_now(duration: Duration) -> Self {
        Self {
            trace_headers: HashMap::new(),
            deadline: Some(tokio::time::Instant::now() + duration),
            metadata: None,
        }
    }

    /// Set or replace the deadline to `duration` from now.
    pub fn set_deadline_from_now(&mut self, duration: Duration) {
        self.deadline = Some(tokio::time::Instant::now() + duration);
    }

    /// Clear any previously set deadline.
    pub fn clear_deadline(&mut self) {
        self.deadline = None;
    }

    /// Returns remaining time until deadline using current monotonic clock.
    /// If the deadline is in the past this returns `Some(Duration::from_millis(0))`.
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
    #[must_use]
    pub fn time_remaining_ms(&self) -> Option<u128> {
        self.time_remaining().map(|d| d.as_millis())
    }

    /// Returns true when the deadline exists and is already expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        match self.deadline {
            Some(dl) => tokio::time::Instant::now() >= dl,
            None => false,
        }
    }

    /// Helper to insert or replace a trace header.
    pub fn insert_trace_header<K: Into<String>, V: Into<String>>(&mut self, k: K, v: V) {
        self.trace_headers.insert(k.into(), v.into());
    }

    /// Helper to fetch a trace header value.
    #[must_use]
    pub fn get_trace_header(&self, k: &str) -> Option<&String> {
        self.trace_headers.get(k)
    }
}
