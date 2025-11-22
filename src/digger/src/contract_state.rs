//! Contract lifecycle state machine and per-contract economic tracking.
//!
//! This module encapsulates the mutable state for each active contract and
//! exposes a lightweight manager for aggregation queries (e.g., which
//! contracts are ready to send hash batches). It enforces economic invariants
//! and progression rules:
//!
//! States:
//! * `PendingStake` → awaiting funding approval.
//! * `StakeApproved` → active execution; hash batches allowed.
//! * `ExecutionComplete` → all milestones fulfilled; terminal state (final
//!   hash batches still permitted).
//!
//! Economic invariants:
//! * `ore_target = torq × robo_stake` (not subtraction; multiplicative value
//!   expansion loop).
//! * Funding (`add_funding`) transitions only from `PendingStake`.
//! * Milestone completion triggers `ExecutionComplete` only when cumulative
//!   count reaches `milestones_total`.
//!
//! Concurrency model: All mutation occurs behind external `Mutex<ContractStateManager>`.
//! The manager itself is not internally synchronized to avoid double-locking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contract approval / lifecycle status.
///
/// Allowed progression: `PendingStake → StakeApproved → ExecutionComplete`.
/// Any attempt to re-fund or re-complete beyond terminal state yields errors.
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

/// Mutable state for a single contract instance.
///
/// Fields track economic inputs (torq, robo_stake), progress (milestones,
/// JTUs, ore generated), and machine specs (`power_watts`). Future phases will
/// extend with multi-robot attribution and richer accountability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    pub contract_id: String,
    
    // TODO(phase-4-crypto): Add robot identity tracking
    // - robot_ids: Vec<String>  // Support multi-robot contracts
    // - robot_contributions: HashMap<String, f64>  // Track ore per robot
    // - Needed for: accountability, multi-robot coordination, dispute resolution
    // - Must cross-check robot_id against robot registry (verify power_watts)
    
    // Economics (CORRECT formula: Ore Target = Torq × RoboStake)
    pub torq: f64,                     // Selling price (goes to citizens)
    pub robo_stake: f64,               // Robot payment (goes to DistoDam Reserve)
    pub robo_stake_received: f64,      // Actual RT received from DistoDam (for funding check)
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
    // TODO(phase-4-crypto): Replace with robot registry lookup
    // - power_watts should come from robot_id → registry query
    // - Each robot in robot_ids has its own power spec
    
    // Timestamps
    pub created_at: i64,               // Contract creation timestamp
}

impl ContractState {
    /// Instantiate a new contract state computing `ore_target = torq × robo_stake`.
    ///
    /// Returns a contract in `PendingStake` awaiting funding approval. No JTU
    /// or milestone progress is present at creation. `created_at` records UTC
    /// timestamp for audit; downstream services may use this for SLA metrics.
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
            robo_stake_received: 0.0,  // No funding yet
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
    
    /// Apply funding (RoboStake) released by upstream distribution (Trust/DistoDam).
    ///
    /// Side effects:
    /// * Increments `robo_stake_received`.
    /// * Transitions `PendingStake → StakeApproved`.
    ///
    /// Errors:
    /// * Funding attempted in non-`PendingStake` state.
    /// * Non-positive funding amount.
    pub fn add_funding(&mut self, amount: f64) -> Result<(), String> {
        if self.approval_status != ApprovalStatus::PendingStake {
            return Err(format!(
                "Contract {} is not in PendingStake state (current: {:?})",
                self.contract_id, self.approval_status
            ));
        }
        
        if amount <= 0.0 {
            return Err("Funding amount must be positive".to_string());
        }
        
        self.robo_stake_received += amount;
        
        // Funding also approves the contract
        self.approval_status = ApprovalStatus::StakeApproved;
        
        Ok(())
    }
    
    /// Returns true if received funding meets or exceeds required robo stake.
    pub fn is_funded(&self) -> bool {
        self.robo_stake_received >= self.robo_stake
    }
    
    /// Determine whether hash batch emission criteria are met.
    ///
    /// Conditions:
    /// 1. Lifecycle in `StakeApproved` or `ExecutionComplete`.
    /// 2. At least one JTU present (`jtu_count > 0`).
    /// 3. Interval since `last_hash_send` ≥ `batch_interval_sec` (or never sent).
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
    
    /// Update `last_hash_send` to current UTC timestamp after successful publish.
    pub fn mark_hash_send(&mut self) {
        self.last_hash_send = Some(chrono::Utc::now().timestamp());
    }
    
    /// Record newly generated JTUs and corresponding ore value contribution.
    /// Assumes caller has already calculated fair ore distribution per unit.
    pub fn add_jtus(&mut self, count: i64, ore_value: f64) {
        self.jtu_count += count;
        self.ore_generated += ore_value;
    }
    
    /// Returns true if cumulative ore meets or exceeds the economic target.
    pub fn is_ore_target_reached(&self) -> bool {
        self.ore_generated >= self.ore_target
    }
    
    /// Mark a single milestone as completed, transitioning to
    /// `ExecutionComplete` when final milestone is reached.
    ///
    /// Errors on overflow (attempting completion beyond `milestones_total`).
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
    
    /// Returns true if lifecycle has reached terminal `ExecutionComplete`.
    pub fn is_complete(&self) -> bool {
        self.approval_status == ApprovalStatus::ExecutionComplete
    }
    
    /// Fractional progress (0.0–1.0) based on milestone completion ratio.
    pub fn progress(&self) -> f64 {
        if self.milestones_total == 0 {
            return 0.0;
        }
        self.milestones_completed as f64 / self.milestones_total as f64
    }
}

/// Aggregates and indexes active contracts; provides selection helpers.
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
    
    /// Insert a newly created contract; errors if `contract_id` already present.
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
    
    /// Retrieve mutable access to a contract by id.
    pub fn get_mut(&mut self, contract_id: &str) -> Option<&mut ContractState> {
        self.contracts.get_mut(contract_id)
    }
    
    /// Retrieve immutable access to a contract by id.
    pub fn get(&self, contract_id: &str) -> Option<&ContractState> {
        self.contracts.get(contract_id)
    }
    
    /// Collect contract IDs eligible for hash batch emission.
    pub fn contracts_ready_to_send(&self, batch_interval_sec: u64) -> Vec<String> {
        self.contracts
            .iter()
            .filter(|(_, state)| state.should_send_hashes(batch_interval_sec))
            .map(|(id, _)| id.clone())
            .collect()
    }
    
    /// Return a vector of all currently tracked contract IDs.
    pub fn all_contract_ids(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }
    
    /// Remove and return IDs of contracts in terminal `ExecutionComplete` state.
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
    fn test_add_funding_success() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        
        let result = state.add_funding(5.0);
        assert!(result.is_ok());
        assert_eq!(state.approval_status, ApprovalStatus::StakeApproved);
        assert_eq!(state.robo_stake_received, 5.0);
    }

    #[test]
    fn test_add_funding_already_approved() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        state.add_funding(5.0).unwrap();
        
        // Try to fund again
        let result = state.add_funding(5.0);
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
        state.add_funding(5.0).unwrap();
        
        // Now should send
        assert!(state.should_send_hashes(60));
    }

    #[test]
    fn test_should_send_hashes_time_interval() {
        let mut state = ContractState::new("test-001".to_string(), 100.0, 5.0, 5, 2000.0);
        state.add_funding(5.0).unwrap();
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
        state.add_funding(5.0).unwrap();
        
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
        state.add_funding(5.0).unwrap();
        
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
        state.add_funding(5.0).unwrap();
        
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
        manager.get_mut("test-001").unwrap().add_funding(5.0).unwrap();
        manager.get_mut("test-001").unwrap().add_jtus(100, 50.0);
        
        manager.get_mut("test-002").unwrap().add_funding(5.0).unwrap();
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
        manager.get_mut("test-001").unwrap().add_funding(5.0).unwrap();
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
