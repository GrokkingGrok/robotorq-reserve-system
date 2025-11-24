use async_nats::Client as NatsClient;
use crate::ShadowStakeVault;
use crate::VaultMetrics;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingContract {
    pub id: String,
    pub builder: String,
    pub description: String,
    pub digger_url: String,
    pub required_rt: i64,
    pub roi: f64,
    pub submitted_at: i64,
    // Future: torq_factor, priority_score, etc.
}

#[derive(Clone, Debug)]
pub struct ApprovalConfig {
    pub min_available_stake_ratio: f64,
    pub max_concurrent_contracts: usize,
}

pub struct ContractApproval {
    stake_vault: Arc<ShadowStakeVault>,
    nats: NatsClient,
    config: ApprovalConfig,
    metrics: Arc<VaultMetrics>,
    active_contracts: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl ContractApproval {
    pub fn new(
        stake_vault: Arc<ShadowStakeVault>,
        nats: NatsClient,
        config: ApprovalConfig,
        metrics: Arc<VaultMetrics>,
    ) -> Self {
        Self {
            stake_vault,
            nats,
            config,
            metrics,
            active_contracts: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Check if a contract can be approved based on current vault state
    pub async fn can_approve(&self, contract: &PendingContract) -> bool {
        // Check if we have enough available stake
        let available = self.stake_vault.available_robostake();
        let total_stake = available + self.stake_vault.deployed_robostake();
        let min_required = (total_stake as f64 * self.config.min_available_stake_ratio) as i64;

        if available < contract.required_rt + min_required {
            warn!(
                contract_id = %contract.id,
                required = contract.required_rt,
                available = available,
                min_required = min_required,
                "Insufficient stake for contract approval"
            );
            return false;
        }

        // Check concurrent contract limit
        let active_count = self.active_contracts.lock().unwrap().len();
        if active_count >= self.config.max_concurrent_contracts {
            warn!(
                contract_id = %contract.id,
                active_count = active_count,
                max_allowed = self.config.max_concurrent_contracts,
                "Too many concurrent contracts"
            );
            return false;
        }

        // Future: Check liabilities, torq factors, etc.
        // For now, basic checks only

        true
    }

    /// Approve and fund a contract
    pub async fn approve_contract(&self, contract: PendingContract) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.can_approve(&contract).await {
            return Err("Contract cannot be approved".into());
        }

        // Record as active
        self.active_contracts.lock().unwrap().insert(contract.id.clone());

        // Allocate stake (deploy it)
        self.stake_vault.allocate(&contract.id, contract.required_rt).await?;

        // Publish funded event
        let funded_event = serde_json::json!({
            "event_type": "contract_funded",
            "contract_id": contract.id,
            "builder": contract.builder,
            "required_rt": contract.required_rt,
            "funded_at": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats.publish("contracts.funded", serde_json::to_vec(&funded_event)?.into()).await?;

        self.metrics.inc_contracts_approved();
        info!(contract_id = %contract.id, required_rt = contract.required_rt, "Contract approved and funded");

        Ok(())
    }

    /// Handle contract completion (called when contract finishes)
    pub async fn complete_contract(&self, contract_id: &str) {
        self.active_contracts.lock().unwrap().remove(contract_id);
        self.metrics.inc_contracts_completed();
        info!(contract_id = %contract_id, "Contract marked as completed");
    }

    /// Start the approval service (subscribe to pending contracts)
    pub async fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut subscriber = self.nats.subscribe("contracts.pending").await?;
        info!("Contract approval service started, listening for pending contracts");

        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                match serde_json::from_slice::<PendingContract>(&msg.payload) {
                    Ok(contract) => {
                        self.metrics.inc_contracts_received();
                        match self.approve_contract(contract).await {
                            Ok(()) => {}
                            Err(e) => error!(error = %e, "Failed to approve contract"),
                        }
                    }
                    Err(e) => error!(error = %e, "Invalid contract payload"),
                }
            }
        });

        Ok(())
    }
}