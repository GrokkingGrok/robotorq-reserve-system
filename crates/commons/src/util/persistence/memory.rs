use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::context::Context;
use super::error::PersistenceError;
use super::traits::{PersistenceDriver, PersistenceHealth};

/// Simple thread-safe in-memory persistence driver used for tests and local development.
///
/// Provides minimal CRUD operations on a string key/value map. The store is
/// protected by a `Mutex` and is `Clone` via internal `Arc`, making it convenient
/// to share across test tasks.
#[derive(Debug, Clone)]
pub struct InMemoryDriver {
    ready: bool,
    store: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for InMemoryDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDriver {
    /// Create a new in-memory driver instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ready: true,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Begin a transaction scoped to this driver. Changes are local to the
    /// transaction until `commit()` is called.
    #[must_use]
    pub fn begin_transaction(&self) -> InMemoryTransaction {
        InMemoryTransaction {
            store: self.store.clone(),
            staged: HashMap::new(),
            committed: false,
        }
    }

    /// Create a checkpoint snapshot of the current store state.
    #[must_use]
    pub fn create_checkpoint(&self) -> HashMap<String, String> {
        match self.store.lock() {
            Ok(g) => g.clone(),
            Err(_) => HashMap::new(),
        }
    }

    /// Restore the store state from a checkpoint snapshot, replacing current contents.
    pub fn restore_checkpoint(
        &self,
        snapshot: HashMap<String, String>,
    ) -> Result<(), PersistenceError> {
        let mut g = self
            .store
            .lock()
            .map_err(|e| PersistenceError::Internal(format!("lock error: {e}")))?;
        *g = snapshot;
        Ok(())
    }

    /// Seed multiple entries into the store (helper for tests).
    pub fn seed<I, K, V>(&self, iter: I) -> Result<(), PersistenceError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut g = self
            .store
            .lock()
            .map_err(|e| PersistenceError::Internal(format!("lock error: {e}")))?;
        for (k, v) in iter {
            g.insert(k.into(), v.into());
        }
        Ok(())
    }

    /// Insert or update a value for `key`.
    pub fn put(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        let mut g = self
            .store
            .lock()
            .map_err(|e| PersistenceError::Internal(format!("lock error: {e}")))?;
        g.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Get a value by `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        let g = self.store.lock().ok()?;
        g.get(key).cloned()
    }

    /// Delete the entry for `key`, returning `true` if it existed.
    #[must_use]
    pub fn delete(&self, key: &str) -> bool {
        let Ok(mut g) = self.store.lock() else { return false };
        g.remove(key).is_some()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        if let Ok(mut g) = self.store.lock() {
            g.clear();
        }
    }
}

#[async_trait::async_trait]
impl PersistenceDriver for InMemoryDriver {
    fn health(&self) -> PersistenceHealth {
        PersistenceHealth {
            ready: self.ready,
            message: None,
        }
    }

    fn shutdown(&self) {}

    async fn ping(&self, ctx: &Context) -> Result<(), PersistenceError> {
        if ctx.is_expired() {
            Err(PersistenceError::DeadlineExceeded)
        } else {
            Ok(())
        }
    }
}

/// Transaction object returned by `InMemoryDriver::begin_transaction()`.
pub struct InMemoryTransaction {
    store: Arc<Mutex<HashMap<String, String>>>,
    staged: HashMap<String, Option<String>>,
    committed: bool,
}

impl InMemoryTransaction {
    /// Put a value into the transaction's staged changes.
    pub fn put(&mut self, key: &str, value: &str) {
        self.staged.insert(key.to_string(), Some(value.to_string()));
    }

    /// Delete a key in the transaction's staged changes.
    pub fn delete(&mut self, key: &str) {
        self.staged.insert(key.to_string(), None);
    }

    /// Get a value: prefers staged changes, falls back to the live store.
    pub fn get(&self, key: &str) -> Option<String> {
        if let Some(opt) = self.staged.get(key) {
            return opt.clone();
        }
        let g = self.store.lock().ok()?;
        g.get(key).cloned()
    }

    /// Commit staged changes into the live store.
    pub fn commit(mut self) -> Result<(), PersistenceError> {
        let mut g = self
            .store
            .lock()
            .map_err(|e| PersistenceError::Internal(format!("lock error: {e}")))?;
        for (k, v) in self.staged.drain() {
            match v {
                Some(s) => {
                    g.insert(k, s);
                }
                None => {
                    g.remove(&k);
                }
            }
        }
        self.committed = true;
        Ok(())
    }

    /// Abort the transaction without applying staged changes.
    pub fn abort(mut self) {
        self.staged.clear();
        self.committed = true;
    }
}

impl Drop for InMemoryTransaction {
    fn drop(&mut self) {
        // If transaction was not committed, just drop staged changes (abort).
        if !self.committed {
            self.staged.clear();
        }
    }
}
