//! Persistence driver traits and health reporting.
//!
//! This module defines the common interface that all persistence backends
//! (SQLite, Postgres, in‑memory) implement so services can talk to storage
//! in a consistent, easy‑to‑understand way. The focus is on simplicity:
//! a small health struct, a driver trait, and an optional async `ping`.
//!
//! The docs here are written to be approachable — imagine explaining
//! how to check if a database is "ready" to a curious high‑schooler.
//! You ask the driver "are you ready?" (`health()`), and it answers
//! with `ready = true/false` and a short message.

use super::context::Context;
use super::error::PersistenceError;
use async_trait::async_trait;

/// Minimal health view for readiness gates.
///
/// Think of this like a simple "status card". It says if the driver
/// is ready and can include a short note.
///
/// # Fields
/// - `ready`: `true` when the driver is initialized and the schema
///   looks compatible; `false` otherwise.
/// - `message`: optional note describing the health state (for humans).
///
/// # Examples
/// ```
/// use commons::util::persistence::PersistenceHealth;
/// let h = PersistenceHealth { ready: true, message: Some("ok".to_string()) };
/// assert!(h.ready);
/// ```
#[derive(Clone, Debug)]
pub struct PersistenceHealth {
    /// True when driver is initialized and schema is compatible.
    pub ready: bool,
    /// Optional message describing health state.
    pub message: Option<String>,
}

/// Common interface for persistence backends.
///
/// Backends implement this trait to expose a unified set of operations.
/// Each method below includes a short description and, where useful,
/// an example.
///
/// # Examples
/// ```no_run
/// # use commons::util::persistence::{PersistenceDriver, PersistenceHealth};
/// struct MemoryDriver;
/// #[async_trait::async_trait]
/// impl PersistenceDriver for MemoryDriver {
///     fn health(&self) -> PersistenceHealth { PersistenceHealth { ready: true, message: None } }
/// }
/// # let d = MemoryDriver; let h = d.health(); assert!(h.ready);
/// ```
#[async_trait]
pub trait PersistenceDriver: Send + Sync + 'static {
    /// Returns current health; should be cheap and non‑blocking.
    ///
    /// # Returns
    /// - `PersistenceHealth`: quick snapshot of readiness.
    fn health(&self) -> PersistenceHealth;

    /// Optional per‑request health validation.
    ///
    /// Some drivers can perform a stricter check on demand (for example,
    /// verifying a live connection). By default this returns `health()`.
    ///
    /// # Returns
    /// - `PersistenceHealth`: on‑demand snapshot of readiness.
    async fn health_now(&self) -> PersistenceHealth {
        self.health()
    }

    /// Optional shutdown for drivers that hold resources.
    ///
    /// Drivers that manage connections can override this to close them.
    /// The default does nothing.
    fn shutdown(&self) {}

    /// Async ping operation to verify liveliness.
    ///
    /// This method may use a lightweight query to check that the backend
    /// responds in time. Implementations should respect `Context` deadlines.
    ///
    /// # Arguments
    /// - `_ctx`: the operation context with an optional monotonic deadline.
    ///
    /// # Returns
    /// - `Ok(())` if the driver is responsive
    /// - `Err(PersistenceError)` if the backend is unavailable or timed out
    ///
    /// # Examples
    /// ```no_run
    /// # use commons::util::persistence::{PersistenceDriver, PersistenceHealth};
    /// # use commons::util::persistence::Context;
    /// # struct MyDriver;
    /// # #[async_trait::async_trait]
    /// # impl PersistenceDriver for MyDriver { fn health(&self) -> PersistenceHealth { PersistenceHealth { ready: true, message: None } } }
    /// # async fn demo(drv: &MyDriver) -> Result<(), commons::util::persistence::PersistenceError> {
    /// let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(50));
    /// // A real driver would run a tiny query here
    /// let _ = drv.ping(&ctx).await?;
    /// # Ok(()) }
    /// ```
    async fn ping(&self, _ctx: &Context) -> Result<(), PersistenceError> {
        Ok(())
    }
}
