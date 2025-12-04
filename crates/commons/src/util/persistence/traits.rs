//! Persistence driver traits and health reporting.

use super::context::Context;
use super::error::PersistenceError;
use async_trait::async_trait;

/// Minimal health view for readiness gates.
#[derive(Clone, Debug)]
pub struct PersistenceHealth {
    /// True when driver is initialized and schema is compatible.
    pub ready: bool,
    /// Optional message describing health state.
    pub message: Option<String>,
}

/// Common interface for persistence backends.
#[async_trait]
pub trait PersistenceDriver: Send + Sync + 'static {
    /// Returns current health; should be cheap and non-blocking.
    fn health(&self) -> PersistenceHealth;

    /// Optional shutdown for drivers that hold resources.
    fn shutdown(&self) {}

    /// Async ping operation to verify liveliness and respect `Context` deadlines.
    async fn ping(&self, _ctx: &Context) -> Result<(), PersistenceError> {
        Ok(())
    }
}
