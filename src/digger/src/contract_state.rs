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
/// 
/// **CRITICAL ECONOMICS** (differs from whitepaper):
/// - Torq (Selling Price): Total value being created (goes to UBD wallets)
/// - RoboStake: Payment for robot (goes to DistoDam Reserve)
/// - Ore Target: Torq × RoboStake (how much to dig!)
/// 
/// This creates the self-sustaining Robotic Labor Reserve Loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    pub contract_id: String,
    
    // Economics (CORRECT formula: Ore Target = Torq × RoboStake)
    pub torq: f64,                     // Selling price (goes to citizens)
    pub robo_stake: f64,               // Robot payment (goes to DistoDam Reserve)
    pub ore_target: f64,               // Torq × RoboStake (how much to dig)
    pub ore_generated: f64,            // How much dug so far
    
    // State machine
    pub approval_status: ApprovalStatus,
    
    // Work tracking
    pub jtu_count: i64,
    pub last_hash_send: Option<i64>,  // Unix timestamp of last NATS batch
    pub milestones_total: i64,
    pub milestones_completed: i64,
    
    // Robot specs
    pub power_watts: f64,              // Robot's power (for cross product)
    
    // Timestamps
    pub created_at: i64,               // Contract creation timestamp
}

impl ContractState {
    /// Create new contract with correct economics
    /// 
    /// **CRITICAL**: Ore target = Torq × RoboStake (not Torq - RoboStake!)
    /// 
    /// Example:
    /// - Torq: 100 RT (selling price)
    /// - RoboStake: 5 RT (5% of torq)
    /// - Ore Target: 100 × 5 = 500 RT worth of ore
    /// 
    /// Distribution after minting:
    /// - 100 RT → UBD wallets (Disto)
    /// - 5 RT → DistoDam Reserve (robot payment pool)
    pub fn new(
        contract_id: String,
        torq: f64,
        robo_stake: f64,
        milestones_total: i64,
        power_watts: f64,
    ) -> Self {
        let ore_target = torq * robo_stake;  // CORRECT formula!
        
        Self {
            contract_id,
            torq,
            robo_stake,
            ore_target,
            ore_generated: 0.0,
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
    pub fn pay_stake(&mut self) -> Result<(), String> {
        if self.approval_status != ApprovalStatus::PendingStake {
            return Err(format!(
                "Contract {} is not in PendingStake state (current: {:?})",
                self.contract_id, self.approval_status
            ));
        }
        
        if self.robo_stake <= 0.0 {
            return Err("RoboStake must be positive".to_string());
        }
        
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
    
    /// Increment JTU count and ore generated (when new JTUs are created)
    /// 
    /// **Each JTU represents work done**, contributing to ore generation.
    pub fn add_jtus(&mut self, count: i64, ore_value: f64) {
        self.jtu_count += count;
        self.ore_generated += ore_value;
    }
    
    /// Check if ore target reached
    pub fn is_ore_target_reached(&self) -> bool {
        self.ore_generated >= self.ore_target
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
    
    /// Create new contract with economics
    pub fn create_contract(
        &mut self,
        contract_id: String,
        torq: f64,
        robo_stake: f64,
        milestones: i64,
        power_watts: f64,
    ) -> Result<(), String> {
        if self.contracts.contains_key(&contract_id) {
            return Err(format!("Contract {} already exists", contract_id));
        }
        
        let state = ContractState::new(contract_id.clone(), torq, robo_stake, milestones, power_watts);
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
        // Example: $100 RT sale, 5% RoboStake
        let torq = 100.0;
        let robo_stake = 5.0;
        let state = ContractState::new("test-001".to_string(), torq, robo_stake, 5, 2000.0);
        
        assert_eq!(state.contract_id, "test-001");
        assert_eq!(state.torq, 100.0);
        assert_eq!(state.robo_stake, 5.0);
        assert_eq!(state.ore_target, 500.0);  // 100 × 5 = 500 RT
        assert_eq!(state.ore_generated, 0.0);
        assert_eq!(state.approval_status, ApprovalStatus::PendingStake);
        assert_eq!(state.milestones_total, 5);
        assert_eq!(state.power_watts, 2000.0);
        assert_eq!(state.jtu_count, 0);
    }

    #[test]
    fn test_pay_stake_success() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        
        let result = state.pay_stake();
        assert!(result.is_ok());
        assert_eq!(state.approval_status, ApprovalStatus::StakeApproved);
    }

    #[test]
    fn test_pay_stake_already_approved() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        state.pay_stake().unwrap();
        
        // Try to pay again
        let result = state.pay_stake();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in PendingStake"));
    }

    #[test]
    fn test_should_send_hashes_not_approved() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        state.add_jtus(100, 50.0);  // 100 JTUs worth 50 RT
        
        // Not approved yet
        assert!(!state.should_send_hashes(60));
        
        // Approve
        state.pay_stake().unwrap();
        
        // Now should send
        assert!(state.should_send_hashes(60));
    }

    #[test]
    fn test_should_send_hashes_time_interval() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        state.pay_stake().unwrap();
        state.add_jtus(100, 50.0);
        
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
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 3, 2000.0);
        state.pay_stake().unwrap();
        
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
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 1, 2000.0);
        state.pay_stake().unwrap();
        
        state.complete_milestone().unwrap();
        
        // Try to complete again
        let result = state.complete_milestone();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already completed"));
    }

    #[test]
    fn test_progress() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 4, 2000.0);
        
        assert_eq!(state.progress(), 0.0);
        
        state.milestones_completed = 1;
        assert_eq!(state.progress(), 0.25);
        
        state.milestones_completed = 2;
        assert_eq!(state.progress(), 0.5);
        
        state.milestones_completed = 4;
        assert_eq!(state.progress(), 1.0);
    }
    
    #[test]
    fn test_ore_target_calculation() {
        // Example 1: $100 RT, 5% stake
        let state = ContractState::new("test-001".to_string(), 100.0, 5.0, 10, 2000.0);
        assert_eq!(state.ore_target, 500.0);  // 100 × 5 = 500 RT
        
        // Example 2: $1000 RT, 50 RT stake
        let state2 = ContractState::new("test-002".to_string(), 1000.0, 50.0, 10, 5000.0);
        assert_eq!(state2.ore_target, 50000.0);  // 1000 × 50 = 50,000 RT
    }
    
    #[test]
    fn test_ore_generation_tracking() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 10, 2000.0);
        state.pay_stake().unwrap();
        
        // Ore target is 500 RT
        assert_eq!(state.ore_target, 500.0);
        assert_eq!(state.ore_generated, 0.0);
        assert!(!state.is_ore_target_reached());
        
        // Generate 250 RT worth
        state.add_jtus(1000, 250.0);
        assert_eq!(state.ore_generated, 250.0);
        assert!(!state.is_ore_target_reached());
        
        // Generate another 250 RT worth
        state.add_jtus(1000, 250.0);
        assert_eq!(state.ore_generated, 500.0);
        assert!(state.is_ore_target_reached());  // Target reached!
    }

    #[test]
    fn test_contract_state_manager_create() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 100.0, 5.0, 5, 2000.0).unwrap();
        
        let state = manager.get("test-001").unwrap();
        assert_eq!(state.contract_id, "test-001");
        assert_eq!(state.ore_target, 500.0);
    }

    #[test]
    fn test_contract_state_manager_duplicate() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 100.0, 5.0, 5, 2000.0).unwrap();
        
        let result = manager.create_contract("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_contracts_ready_to_send() {
        let mut manager = ContractStateManager::new();
        
        // Create three contracts
        manager.create_contract("test-001".to_string(), 100.0, 5.0, 5, 2000.0).unwrap();
        manager.create_contract("test-002".to_string(), 100.0, 5.0, 5, 2000.0).unwrap();
        manager.create_contract("test-003".to_string(), 100.0, 5.0, 5, 2000.0).unwrap();
        
        // Approve and add JTUs to first two
        manager.get_mut("test-001").unwrap().pay_stake().unwrap();
        manager.get_mut("test-001").unwrap().add_jtus(100, 50.0);
        
        manager.get_mut("test-002").unwrap().pay_stake().unwrap();
        manager.get_mut("test-002").unwrap().add_jtus(100, 50.0);
        
        // Third is not approved
        
        let ready = manager.contracts_ready_to_send(60);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&"test-001".to_string()));
        assert!(ready.contains(&"test-002".to_string()));
    }

    #[test]
    fn test_remove_completed() {
        let mut manager = ContractStateManager::new();
        
        manager.create_contract("test-001".to_string(), 100.0, 5.0, 1, 2000.0).unwrap();
        manager.create_contract("test-002".to_string(), 100.0, 5.0, 2, 2000.0).unwrap();
        
        // Complete first contract
        manager.get_mut("test-001").unwrap().pay_stake().unwrap();
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
