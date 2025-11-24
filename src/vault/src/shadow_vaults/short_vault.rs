use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use async_nats::Client;
use crate::metrics::VaultMetrics;
use crate::events::subjects;
use tracing::{info, error};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// ShortVault represents a demurrage-free reserve that wallets draw from via scheduled drips.
/// Wallets apply demurrage to their own balances and request releases from the reserve.
pub struct ShortVault {
    user_id: String,
    balance_canonical_jouletorq: AtomicI64, // Demurrage-free reserve balance
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
    minimum_balance: AtomicI64, // Minimum balance to maintain for drips
    active_drips: Arc<Mutex<HashMap<String, DripSchedule>>>, // Active drip schedules
}

/// Represents a scheduled drip release to a wallet
#[derive(Clone, Debug)]
pub struct DripSchedule {
    pub drip_id: String,
    pub total_amount: i64,
    pub remaining_amount: i64,
    pub drip_rate_per_hour: f64, // Amount to release per hour
    pub start_time: i64, // Unix timestamp when drip started
    pub end_time: i64,   // Unix timestamp when drip should end
    pub last_release_time: i64, // Last time funds were released
}

impl ShortVault {
    pub fn new(user_id: String, nats: Client) -> Self {
        Self {
            user_id,
            balance_canonical_jouletorq: AtomicI64::new(0),
            nats,
            metrics: None,
            minimum_balance: AtomicI64::new(0), // Start with no minimum requirement
            active_drips: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<VaultMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Get current balance in canonical jouletorq
    pub fn balance(&self) -> i64 {
        self.balance_canonical_jouletorq.load(Ordering::SeqCst)
    }

    /// Get the user ID for this vault
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Set minimum balance requirement
    pub fn set_minimum_balance(&self, minimum: i64) {
        self.minimum_balance.store(minimum, Ordering::SeqCst);
        info!(user_id=%self.user_id, minimum_balance=minimum, "Minimum balance updated");
    }

    /// Get minimum balance requirement
    pub fn minimum_balance(&self) -> i64 {
        self.minimum_balance.load(Ordering::SeqCst)
    }

    /// Get available balance (total - minimum required)
    pub fn available_balance(&self) -> i64 {
        let total = self.balance();
        let minimum = self.minimum_balance();
        total.saturating_sub(minimum)
    }

    /// Credit UBD to this vault (from DistoVault) - demurrage-free reserve
    pub async fn credit_ubd(&self, canonical_jouletorq: i64) -> Result<()> {
        let old_balance = self.balance_canonical_jouletorq.fetch_add(canonical_jouletorq, Ordering::SeqCst);
        let new_balance = old_balance + canonical_jouletorq;

        info!(user_id=%self.user_id, credited=canonical_jouletorq, new_balance=new_balance, "UBD credited to ShortVault reserve");

        if let Some(m) = &self.metrics {
            m.set_short_vault_balance(new_balance);
        }

        // Publish credit event
        let event = serde_json::json!({
            "event_type": "short_vault_ubd_credited",
            "user_id": self.user_id,
            "canonical_jouletorq_credited": canonical_jouletorq,
            "new_balance": new_balance,
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats.publish(subjects::SHORTVAULT_UBD_CREDITED, serde_json::to_vec(&event)?.into()).await?;

        Ok(())
    }

    /// Handle demurrage release request from wallet
    /// Wallet requests a specific amount to be released over a configurable duration
    pub async fn request_demurrage_release(&self, amount: i64, drip_duration_hours: f64, time_compression: f64) -> Result<String> {
        let current_balance = self.balance();
        let available = self.available_balance();

        // Check if we have enough available balance (above minimum)
        if amount > available {
            let response = serde_json::json!({
                "request_id": "denied",
                "user_id": self.user_id,
                "requested_amount": amount,
                "available_amount": available,
                "reason": "insufficient_available_balance",
                "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            });
            self.nats.publish(subjects::SHORTVAULT_DEMURRAGE_RESPONSE, serde_json::to_vec(&response)?.into()).await?;
            return Err(anyhow::anyhow!("Insufficient available balance: requested {}, available {}", amount, available));
        }

        // Create drip schedule
        let drip_id = format!("drip-{}-{}", self.user_id, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let now = chrono::Utc::now().timestamp();
        
        // Configurable drip duration in seconds, adjusted for time compression
        let drip_duration_seconds = (drip_duration_hours * 3600.0) / time_compression;
        let end_time = now + drip_duration_seconds as i64;
        
        // For now, use uniform distribution (constant rate)
        // Future: support different algorithms based on config
        let drip_rate_per_hour = amount as f64 / drip_duration_hours;

        let schedule = DripSchedule {
            drip_id: drip_id.clone(),
            total_amount: amount,
            remaining_amount: amount,
            drip_rate_per_hour,
            start_time: now,
            end_time,
            last_release_time: now,
        };

        // Add to active drips
        {
            let mut drips = self.active_drips.lock().await;
            drips.insert(drip_id.clone(), schedule);
            
            // Update metrics
            if let Some(m) = &self.metrics {
                m.set_active_drips(drips.len() as i64);
            }
        }

        info!(user_id=%self.user_id, drip_id=%drip_id, amount=amount, duration_hours=drip_duration_hours, drip_rate_per_hour=drip_rate_per_hour, "Demurrage release scheduled");

        // Respond to wallet
        let response = serde_json::json!({
            "request_id": drip_id,
            "user_id": self.user_id,
            "approved_amount": amount,
            "drip_duration_seconds": drip_duration_seconds as i64,
            "drip_rate_per_hour": drip_rate_per_hour,
            "start_time": now,
            "end_time": end_time,
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats.publish(subjects::SHORTVAULT_DEMURRAGE_RESPONSE, serde_json::to_vec(&response)?.into()).await?;

        Ok(drip_id)
    }

    /// Process drip releases - called periodically to release funds to wallet
    pub async fn process_drips(&self) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let mut drips_to_remove = Vec::new();
        let mut total_released = 0i64;

        {
            let mut drips = self.active_drips.lock().await;
            
            for (drip_id, schedule) in drips.iter_mut() {
                if now >= schedule.end_time {
                    // Drip complete - release remaining amount
                    if schedule.remaining_amount > 0 {
                        self.balance_canonical_jouletorq.fetch_sub(schedule.remaining_amount, Ordering::SeqCst);
                        total_released += schedule.remaining_amount;
                        
                        // Notify wallet of final release
                        let event = serde_json::json!({
                            "drip_id": drip_id,
                            "user_id": self.user_id,
                            "released_amount": schedule.remaining_amount,
                            "remaining_amount": 0,
                            "is_final": true,
                            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                        });
                        self.nats.publish(subjects::SHORTVAULT_DRIP_RELEASED, serde_json::to_vec(&event)?.into()).await?;
                        
                        info!(user_id=%self.user_id, drip_id=%drip_id, released=schedule.remaining_amount, "Final drip release");
                    }
                    drips_to_remove.push(drip_id.clone());
                } else {
                    // Calculate how much should be released since last check
                    let hours_elapsed = (now - schedule.last_release_time) as f64 / 3600.0;
                    let amount_to_release = (schedule.drip_rate_per_hour * hours_elapsed) as i64;
                    
                    if amount_to_release > 0 && amount_to_release <= schedule.remaining_amount {
                        self.balance_canonical_jouletorq.fetch_sub(amount_to_release, Ordering::SeqCst);
                        schedule.remaining_amount -= amount_to_release;
                        schedule.last_release_time = now;
                        total_released += amount_to_release;
                        
                        // Notify wallet of incremental release
                        let event = serde_json::json!({
                            "drip_id": drip_id,
                            "user_id": self.user_id,
                            "released_amount": amount_to_release,
                            "remaining_amount": schedule.remaining_amount,
                            "is_final": false,
                            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                        });
                        self.nats.publish(subjects::SHORTVAULT_DRIP_RELEASED, serde_json::to_vec(&event)?.into()).await?;
                        
                        info!(user_id=%self.user_id, drip_id=%drip_id, released=amount_to_release, remaining=schedule.remaining_amount, "Incremental drip release");
                    }
                }
            }
            
            // Remove completed drips
            for drip_id in drips_to_remove {
                drips.remove(&drip_id);
                
                // Update metrics for completed drip
                if let Some(m) = &self.metrics {
                    m.inc_drips_completed();
                    m.set_active_drips(drips.len() as i64);
                }
            }
        }

        if total_released > 0 {
            if let Some(m) = &self.metrics {
                m.inc_wallet_transfers(total_released);
            }
        }

        Ok(())
    }

    /// Handle wallet balance replenishment
    pub async fn replenish_balance(&self, amount: i64) -> Result<()> {
        let old_balance = self.balance_canonical_jouletorq.fetch_add(amount, Ordering::SeqCst);
        let new_balance = old_balance + amount;

        info!(user_id=%self.user_id, replenished=amount, new_balance=new_balance, "Wallet replenished ShortVault balance");

        if let Some(m) = &self.metrics {
            m.set_short_vault_balance(new_balance);
        }

        // Publish replenishment event
        let event = serde_json::json!({
            "event_type": "short_vault_balance_replenished",
            "user_id": self.user_id,
            "canonical_jouletorq_replenished": amount,
            "new_balance": new_balance,
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats.publish(subjects::SHORTVAULT_UBD_CREDITED, serde_json::to_vec(&event)?.into()).await?;

        Ok(())
    }

    /// Get statistics about active drips for monitoring
    pub async fn get_drip_stats(&self) -> serde_json::Value {
        let drips = self.active_drips.lock().await;
        let now = chrono::Utc::now().timestamp();
        
        let mut total_scheduled = 0i64;
        let mut total_remaining = 0i64;
        
        for schedule in drips.values() {
            total_scheduled += schedule.total_amount;
            total_remaining += schedule.remaining_amount;
        }
        
        serde_json::json!({
            "user_id": self.user_id,
            "active_drips": drips.len(),
            "total_scheduled_amount": total_scheduled,
            "total_remaining_amount": total_remaining,
            "current_timestamp": now,
        })
    }
}

/// ShortVaultRegistry manages all user ShortVaults
#[derive(Clone)]
pub struct ShortVaultRegistry {
    vaults: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<ShortVault>>>>,
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
}

impl ShortVaultRegistry {
    pub fn new(nats: Client) -> Self {
        Self {
            vaults: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            nats,
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<VaultMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Get or create a ShortVault for a user
    pub async fn get_or_create_vault(&self, user_id: &str) -> Result<Arc<ShortVault>> {
        // First check if vault exists without holding lock across await
        {
            let vaults = self.vaults.lock().unwrap();
            if let Some(vault) = vaults.get(user_id) {
                return Ok(vault.clone());
            }
        }

        // Vault doesn't exist, create it
        let vault = Arc::new(ShortVault::new(
            user_id.to_string(),
            self.nats.clone(),
        ).with_metrics(self.metrics.clone().unwrap()));

        // Publish creation event
        let event = serde_json::json!({
            "event_type": "short_vault_created",
            "user_id": user_id,
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats.publish(subjects::SHORTVAULT_CREATED, serde_json::to_vec(&event)?.into()).await?;

        info!(user_id=%user_id, "ShortVault created");

        // Insert into map
        {
            let mut vaults = self.vaults.lock().unwrap();
            vaults.insert(user_id.to_string(), vault.clone());
        }

        Ok(vault)
    }

    /// Get all existing vaults (for UBD distribution)
    pub fn get_all_vaults(&self) -> Vec<Arc<ShortVault>> {
        self.vaults.lock().unwrap().values().cloned().collect()
    }

    /// Apply demurrage to all vaults (called periodically)
    /// In simulation mode, this can be called more frequently
    pub async fn apply_demurrage_to_all(&self) -> Result<()> {
        let vaults = self.get_all_vaults();
        for vault in vaults {
            if let Err(e) = vault.process_drips().await {
                error!(user_id=%vault.user_id, error=%e, "Failed to process drips");
            }
        }
        Ok(())
    }

    /// Get total balance across all vaults (for economic monitoring)
    pub fn total_balance(&self) -> i64 {
        self.get_all_vaults().iter().map(|v| v.balance()).sum()
    }

    /// Get aggregate drip statistics across all vaults
    pub async fn get_aggregate_drip_stats(&self) -> serde_json::Value {
        let vaults = self.get_all_vaults();
        let mut total_active_drips = 0usize;
        let mut total_scheduled = 0i64;
        let mut total_remaining = 0i64;
        
        for vault in vaults {
            let stats: serde_json::Value = vault.get_drip_stats().await;
            if let (Some(active), Some(scheduled), Some(remaining)) = (
                stats.get("active_drips").and_then(|v| v.as_u64()),
                stats.get("total_scheduled_amount").and_then(|v| v.as_i64()),
                stats.get("total_remaining_amount").and_then(|v| v.as_i64()),
            ) {
                total_active_drips += active as usize;
                total_scheduled += scheduled;
                total_remaining += remaining;
            }
        }
        
        serde_json::json!({
            "total_vaults": self.get_all_vaults().len(),
            "total_active_drips": total_active_drips,
            "total_scheduled_amount": total_scheduled,
            "total_remaining_amount": total_remaining,
            "current_timestamp": chrono::Utc::now().timestamp(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_nats::connect;
    use std::sync::Arc;

    // Helper function to create a mock NATS client for testing
    async fn create_test_nats() -> Client {
        // For unit tests, we'll use a mock or skip NATS-dependent tests
        // In a real scenario, you'd want to set up a test NATS server
        connect("nats://127.0.0.1:4222").await.expect("Test NATS server required")
    }

    #[tokio::test]
    async fn test_short_vault_creation() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        assert_eq!(vault.user_id(), "test-user");
        assert_eq!(vault.balance(), 0);
        assert_eq!(vault.minimum_balance(), 0);
        assert_eq!(vault.available_balance(), 0);
    }

    #[tokio::test]
    async fn test_minimum_balance() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        vault.set_minimum_balance(1000);
        assert_eq!(vault.minimum_balance(), 1000);
        assert_eq!(vault.available_balance(), -1000); // 0 - 1000

        // Credit some balance
        vault.balance_canonical_jouletorq.store(2000, Ordering::SeqCst);
        assert_eq!(vault.available_balance(), 1000); // 2000 - 1000
    }

    #[tokio::test]
    async fn test_ubd_credit() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        let result = vault.credit_ubd(1000).await;
        assert!(result.is_ok());
        assert_eq!(vault.balance(), 1000);
    }

    #[tokio::test]
    async fn test_demurrage_release_insufficient_balance() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        // Set minimum balance to 500
        vault.set_minimum_balance(500);
        // Credit only 300 (below minimum)
        vault.balance_canonical_jouletorq.store(300, Ordering::SeqCst);

        let result = vault.request_demurrage_release(200, 1.0, 1.0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient available balance"));
    }

    #[tokio::test]
    async fn test_demurrage_release_success() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        // Credit sufficient balance
        vault.balance_canonical_jouletorq.store(2000, Ordering::SeqCst);

        let result = vault.request_demurrage_release(1000, 2.0, 1.0).await;
        assert!(result.is_ok());

        let drip_id = result.unwrap();
        assert!(drip_id.starts_with("drip-test-user-"));

        // Check that drip was created
        let stats = vault.get_drip_stats().await;
        assert_eq!(stats["active_drips"], 1);
        assert_eq!(stats["total_scheduled_amount"], 1000);
        assert_eq!(stats["total_remaining_amount"], 1000);
    }

    #[tokio::test]
    async fn test_drip_processing() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        // Credit balance and create drip
        vault.balance_canonical_jouletorq.store(2000, Ordering::SeqCst);
        let drip_id = vault.request_demurrage_release(1000, 1.0, 1.0).await.unwrap();

        // Simulate time passing (1 hour)
        {
            let mut drips = vault.active_drips.lock().await;
            if let Some(schedule) = drips.get_mut(&drip_id) {
                schedule.last_release_time = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
            }
        }

        // Process drips
        let result = vault.process_drips().await;
        assert!(result.is_ok());

        // Should have released ~1000 jouletorq (full amount for 1-hour drip)
        assert!(vault.balance() < 2000);
    }

    #[tokio::test]
    async fn test_balance_replenishment() {
        let nats = create_test_nats().await;
        let vault = ShortVault::new("test-user".to_string(), nats);

        let result = vault.replenish_balance(500).await;
        assert!(result.is_ok());
        assert_eq!(vault.balance(), 500);
    }

    #[tokio::test]
    async fn test_registry_vault_creation() {
        let nats = create_test_nats().await;
        let metrics = crate::metrics::VaultMetrics::new();
        let registry = ShortVaultRegistry::new(nats).with_metrics(metrics);

        let vault = registry.get_or_create_vault("test-user").await.unwrap();
        assert_eq!(vault.user_id(), "test-user");

        // Get same vault again
        let vault2 = registry.get_or_create_vault("test-user").await.unwrap();
        assert_eq!(vault.user_id(), vault2.user_id());
    }

    #[tokio::test]
    async fn test_registry_multiple_vaults() {
        let nats = create_test_nats().await;
        let metrics = crate::metrics::VaultMetrics::new();
        let registry = ShortVaultRegistry::new(nats).with_metrics(metrics);

        let vault1 = registry.get_or_create_vault("user1").await.unwrap();
        let vault2 = registry.get_or_create_vault("user2").await.unwrap();

        assert_eq!(vault1.user_id(), "user1");
        assert_eq!(vault2.user_id(), "user2");

        let all_vaults = registry.get_all_vaults();
        assert_eq!(all_vaults.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_total_balance() {
        let nats = create_test_nats().await;
        let metrics = crate::metrics::VaultMetrics::new();
        let registry = ShortVaultRegistry::new(nats).with_metrics(metrics);

        let vault1 = registry.get_or_create_vault("user1").await.unwrap();
        let vault2 = registry.get_or_create_vault("user2").await.unwrap();

        vault1.balance_canonical_jouletorq.store(1000, Ordering::SeqCst);
        vault2.balance_canonical_jouletorq.store(2000, Ordering::SeqCst);

        assert_eq!(registry.total_balance(), 3000);
    }

    #[tokio::test]
    async fn test_drip_schedule_creation() {
        let schedule = DripSchedule {
            drip_id: "test-drip".to_string(),
            total_amount: 1000,
            remaining_amount: 1000,
            drip_rate_per_hour: 500.0,
            start_time: 1000000,
            end_time: 1003600, // 1 hour later
            last_release_time: 1000000,
        };

        assert_eq!(schedule.drip_id, "test-drip");
        assert_eq!(schedule.total_amount, 1000);
        assert_eq!(schedule.remaining_amount, 1000);
        assert_eq!(schedule.drip_rate_per_hour, 500.0);
    }
}