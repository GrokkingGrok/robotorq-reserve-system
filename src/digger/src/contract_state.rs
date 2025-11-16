// Contract State Machine - Lifecycle management for contracts
// Phase 1: Digger Rewrite - Day 3

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contract approval status (state machine)
/// 
/// State transitions:
/// PendingStake → StakeApproved → ExecutionComplete
///     ↓              ↓
///   (error)      (can send hashes)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Contract created, but no RoboStake paid yet
    /// Cannot send hashes to Refinery in this state
    PendingStake,
    
    /// RoboStake paid, approved to send hashes
    /// This is the working state
    StakeApproved,
    
    /// All milestones complete, contract finished
    /// Can still send final hash batch, then archive
    ExecutionComplete,
}

/// Per-contract state
/// 
/// Tracks everything needed to manage one contract's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    pub contract_id: String,
    pub robo_stake_paid: f64,
    pub approval_status: ApprovalStatus,
    pub jtu_count: i64,
    pub last_hash_send: Option<i64>,  // Unix timestamp of last NATS batch
    pub milestones_total: i64,
    pub milestones_completed: i64,
    pub power_watts: f64,              // Robot's power (for cross product)
    pub created_at: i64,               // Contract creation timestamp
}

impl ContractState {
    /// Create new contract in PendingStake state
    pub fn new(contract_id: String, milestones_total: i64, power_watts: f64) -> Self {
        Self {
            contract_id,
            robo_stake_paid: 0.0,
            approval_status: ApprovalStatus::PendingStake,
            jtu_count: 0,
            last_hash_send: None,
            milestones_total,
            milestones_completed: 0,
            power_watts,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Pay RoboStake and approve contract
    /// 
    /// Transitions: PendingStake → StakeApproved
    pub fn pay_stake(&mut self, amount: f64) -> Result<(), String> {
        if self.approval_status != ApprovalStatus::PendingStake {
            return Err(format!(
                "Contract {} is not in PendingStake state (current: {:?})",
                self.contract_id, self.approval_status
            ));
        }
        
        if amount <= 0.0 {
            return Err("Stake amount must be positive".to_string());
        }
        
        self.robo_stake_paid = amount;
        self.approval_status = ApprovalStatus::StakeApproved;
        
        Ok(())
    }
    
    /// Check if we should send hashes now
    /// 
    /// Returns true if:
    /// 1. Contract is StakeApproved or ExecutionComplete
    /// 2. Enough time has passed since last send (batch_interval_sec)
    /// 3. We have at least 1 JTU to send
    pub fn should_send_hashes(&self, batch_interval_sec: u64) -> bool {
        // Must be approved or complete
        if self.approval_status != ApprovalStatus::StakeApproved
            && self.approval_status != ApprovalStatus::ExecutionComplete
        {
            return false;
        }
        
        // Must have JTUs to send
        if self.jtu_count == 0 {
            return false;
        }
        
        // Check time interval
        match self.last_hash_send {
            None => true, // Never sent before, send now
            Some(last) => {
                let now = chrono::Utc::now().timestamp();
                let elapsed = now - last;
                elapsed >= batch_interval_sec as i64
            }
        }
    }
    
    /// Mark that we just sent hashes
    pub fn mark_hash_send(&mut self) {
        self.last_hash_send = Some(chrono::Utc::now().timestamp());
    }
    
    /// Increment JTU count (when new JTUs are created)
    pub fn add_jtus(&mut self, count: i64) {
        self.jtu_count += count;
    }
    
    /// Complete a milestone
    pub fn complete_milestone(&mut self) -> Result<(), String> {
        if self.milestones_completed >= self.milestones_total {
            return Err(format!(
                "Contract {} already completed all {} milestones",
                self.contract_id, self.milestones_total
            ));
        }
        
        self.milestones_completed += 1;
        
        // If all milestones done, transition to ExecutionComplete
        if self.milestones_completed >= self.milestones_total {
            self.approval_status = ApprovalStatus::ExecutionComplete;
        }
        
        Ok(())
    }
    
    /// Check if contract is complete
    pub fn is_complete(&self) -> bool {
        self.approval_status == ApprovalStatus::ExecutionComplete
    }
    
    /// Get progress percentage (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.milestones_total == 0 {
            return 0.0;
        }
        self.milestones_completed as f64 / self.milestones_total as f64
    }
}

/// Contract state manager - Manages all active contracts
#[derive(Debug, Clone, Default)]
pub struct ContractStateManager {
    contracts: HashMap<String, ContractState>,
}

impl ContractStateManager {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }
    
    /// Create new contract
    pub fn create_contract(
        &mut self,
        contract_id: String,
        milestones: i64,
        power_watts: f64,
    ) -> Result<(), String> {
        if self.contracts.contains_key(&contract_id) {
            return Err(format!("Contract {} already exists", contract_id));
        }
        
        let state = ContractState::new(contract_id.clone(), milestones, power_watts);
        self.contracts.insert(contract_id, state);
        
        Ok(())
    }
    
    /// Get contract state (mutable)
    pub fn get_mut(&mut self, contract_id: &str) -> Option<&mut ContractState> {
        self.contracts.get_mut(contract_id)
    }
    
    /// Get contract state (immutable)
    pub fn get(&self, contract_id: &str) -> Option<&ContractState> {
        self.contracts.get(contract_id)
    }
    
    /// Get all contracts that should send hashes now
    pub fn contracts_ready_to_send(&self, batch_interval_sec: u64) -> Vec<String> {
        self.contracts
            .iter()
            .filter(|(_, state)| state.should_send_hashes(batch_interval_sec))
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Get all contract IDs
    pub fn all_contract_ids(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }
    
    /// Remove completed contracts (for cleanup)
    pub fn remove_completed(&mut self) -> Vec<String> {
        let completed: Vec<String> = self
            .contracts
            .iter()
            .filter(|(_, state)| state.is_complete())
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in &completed {
            self.contracts.remove(id);
        }
        
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_state_new() {
        let state = ContractState::new("test-001".to_string(), 5, 2000.0);
        
        assert_eq!(state.contract_id, "test-001");
        assert_eq!(state.approval_status, ApprovalStatus::PendingStake);
        assert_eq!(state.milestones_total, 5);
        assert_eq!(state.power_watts, 2000.0);
        assert_eq!(state.jtu_count, 0);
    }

    #[test]
    fn test_pay_stake_success() {
        let mut state = ContractState::new("test-001".to_string(), 5, 2000.0);
        
        let result = state.pay_stake(0.05);
        assert!(result.is_ok());
        assert_eq!(state.robo_stake_paid, 0.05);
        assert_eq!(state.approval_status, ApprovalStatus::StakeApproved);
    }

    #[test]
    fn test_pay_stake_already_approved() {
        let mut state = ContractState::new("test-001".to_string(), 5, 2000.0);
        state.pay_stake(0.05).unwrap();
        
        // Try to pay again
        let result = state.pay_stake(0.05);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in PendingStake"));
    }

    #[test]
    fn test_should_send_hashes_not_approved() {
        let mut state = ContractState::new("test-001".to_string(), 5, 2000.0);
        state.add_jtus(100);
        
        // Not approved yet
        assert!(!state.should_send_hashes(60));
        
        // Approve
        state.pay_stake(0.05).unwrap();
        
        // Now should send
        assert!(state.should_send_hashes(60));
    }

    #[test]
    fn test_should_send_hashes_time_interval() {
        let mut state = ContractState::new("test-001".to_string(), 5, 2000.0);
        state.pay_stake(0.05).unwrap();
        state.add_jtus(100);
        
        // First send - should be true
        assert!(state.should_send_hashes(60));
        
        // Mark as sent
        state.mark_hash_send();
        
        // Immediately after - should be false (not enough time)
        assert!(!state.should_send_hashes(60));
        
        // With very short interval - should be true
        assert!(state.should_send_hashes(0));
    }

    #[test]
    fn test_complete_milestone() {
        let mut state = ContractState::new("test-001".to_string(), 3, 2000.0);
        state.pay_stake(0.05).unwrap();
        
        // Complete first milestone
        state.complete_milestone().unwrap();
        assert_eq!(state.milestones_completed, 1);
        assert_eq!(state.approval_status, ApprovalStatus::StakeApproved);
        
        // Complete second
        state.complete_milestone().unwrap();
        assert_eq!(state.milestones_completed, 2);
        
        // Complete third (final)
        state.complete_milestone().unwrap();
        assert_eq!(state.milestones_completed, 3);
        assert_eq!(state.approval_status, ApprovalStatus::ExecutionComplete);
        assert!(state.is_complete());
    }

    #[test]
    fn test_complete_milestone_overflow() {
        let mut state = ContractState::new("test-001".to_string(), 1, 2000.0);
        state.pay_stake(0.05).unwrap();
        
        state.complete_milestone().unwrap();
        
        // Try to complete again
        let result = state.complete_milestone();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already completed"));
    }

    #[test]
    fn test_progress() {
        let mut state = ContractState::new("test-001".to_string(), 4, 2000.0);
        
        assert_eq!(state.progress(), 0.0);
        
        state.milestones_completed = 1;
        assert_eq!(state.progress(), 0.25);
        
        state.milestones_completed = 2;
        assert_eq!(state.progress(), 0.5);
        
        state.milestones_completed = 4;
        assert_eq!(state.progress(), 1.0);
    }

    #[test]
    fn test_contract_state_manager_create() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 5, 2000.0).unwrap();
        
        let state = manager.get("test-001").unwrap();
        assert_eq!(state.contract_id, "test-001");
    }

    #[test]
    fn test_contract_state_manager_duplicate() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 5, 2000.0).unwrap();
        
        let result = manager.create_contract("test-001".to_string(), 5, 2000.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_contracts_ready_to_send() {
        let mut manager = ContractStateManager::new();
        
        // Create three contracts
        manager.create_contract("test-001".to_string(), 5, 2000.0).unwrap();
        manager.create_contract("test-002".to_string(), 5, 2000.0).unwrap();
        manager.create_contract("test-003".to_string(), 5, 2000.0).unwrap();
        
        // Approve and add JTUs to first two
        manager.get_mut("test-001").unwrap().pay_stake(0.05).unwrap();
        manager.get_mut("test-001").unwrap().add_jtus(100);
        
        manager.get_mut("test-002").unwrap().pay_stake(0.05).unwrap();
        manager.get_mut("test-002").unwrap().add_jtus(100);
        
        // Third is not approved
        
        let ready = manager.contracts_ready_to_send(60);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"test-001".to_string()));
        assert!(ready.contains(&"test-002".to_string()));
    }

    #[test]
    fn test_remove_completed() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 1, 2000.0).unwrap();
        manager.create_contract("test-002".to_string(), 2, 2000.0).unwrap();
        
        // Complete first contract
        manager.get_mut("test-001").unwrap().pay_stake(0.05).unwrap();
        manager.get_mut("test-001").unwrap().complete_milestone().unwrap();
        
        // Second is not complete
        
        let removed = manager.remove_completed();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], "test-001");
        
        // Only test-002 should remain
        let all = manager.all_contract_ids();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], "test-002");
    }
}
