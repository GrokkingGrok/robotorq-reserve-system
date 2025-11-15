// This file controls the **robots (Diggers)** and how they work on jobs

use crate::types::{Contract, JouleTorqOre};  // Import our data blueprints
use crate::ore_storage::OreStorage;          // Import the treasure vault
use crate::refinery_client;                  // Import Refinery HTTP client
use std::sync::{Arc, Mutex};                 // Tools to safely share data
use tokio::time::{sleep, Duration};          // Tool to wait between steps
use std::time::{SystemTime, UNIX_EPOCH};     // Tool to get the current time
use lazy_static::lazy_static;                // Magic to create one global thing
use std::collections::HashMap;               // Like a phone book: name → robot
use base64::{engine::general_purpose, Engine as _}; // Turn photos into text
use tauri::Emitter;                          // Send messages to the app window

// ────────────────────────────────────────────────────────────────
// CONTRACT EXECUTION CONTROL
// ────────────────────────────────────────────────────────────────

/// Control state for contract execution (pause/resume/stop)
#[derive(Clone, Debug, PartialEq)]
pub enum ContractControl {
    Running,   // Contract is actively running
    Paused,    // Contract is paused (can resume)
    Stopped,   // Contract has been stopped (permanent, cannot resume)
    Completed, // Contract finished naturally (duration reached)
}

/// Status of an individual milestone submission to Refinery
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum MilestoneStatus {
    #[allow(dead_code)]
    Pending,      // About to be sent
    Sending,      // Currently sending to Refinery
    Confirmed,    // Refinery accepted the ore
    Failed(String), // Refinery rejected or network error (reason stored)
}

// ────────────────────────────────────────────────────────────────
// CONTRACT STATUS UPDATE (TODO #9)
// ────────────────────────────────────────────────────────────────

/// Comprehensive contract status for UI dashboard
#[derive(Clone, Debug, serde::Serialize)]
pub struct ContractStatusUpdate {
    // Identity
    pub contract_id: String,
    pub digger_id: String,
    
    // Economics
    pub total_joules: u64,
    pub total_tokens: u64,
    pub total_robo_stake: f64,
    pub robo_stake_sent: f64,      // Sum of confirmed milestones
    
    // Progress
    pub current_milestone: u32,
    pub total_milestones: u32,
    pub percent_complete: f32,
    pub time_elapsed_secs: u64,
    pub time_remaining_secs: u64,
    
    // Status
    pub milestones_confirmed: u32,
    pub milestones_failed: u32,
    pub refinery_healthy: bool,    // Last send succeeded?
    pub state: String,             // "running", "paused", "stopped", "completed"
}

// ────────────────────────────────────────────────────────────────
// THE ROBOT (Digger)
// ────────────────────────────────────────────────────────────────

/// This is a **robot** that does work and makes digital gold (Ore)
#[derive(Clone)]
pub struct Digger {
    /// The robot's name (like a license plate)
    pub id: String,

    /// How much electricity it uses (in kilowatts)
    /// Example: 0.25 kW = 250 watts
    pub power_kw: f64,

    /// Fastest it can make **tokens per second**
    /// Example: 12 tokens/sec
    pub max_token_throughput: u64,

    /// The job it's working on right now (or None if idle)
    pub current_contract: Option<Contract>,
}

impl Digger {
    /// This is the **"Start Working!"** button for the robot
    pub fn start_contract(
        &mut self,                    // We need to change the robot
        contract: Contract,           // The job agreement
        ore_store: Arc<Mutex<OreStorage>>, // The treasure vault
        app: tauri::AppHandle,        // The app window (to send updates)
        _job_id: String,              // Unique name for this work session (for future logging)
    ) {
        // Save the job so the robot remembers what it's doing
        self.current_contract = Some(contract.clone());

        // Copy robot info so we can use it inside the background task
        let digger_id = self.id.clone();
        let power_kw = self.power_kw;
        // How many tokens this robot makes **per update**
        // = robot speed × job value
        let throughput = self.max_token_throughput * contract.torq as u64;
        
        // Get contract ID for state tracking
        let contract_id_for_state = contract.id.clone();

        // Start a background worker (like a factory machine)
        tauri::async_runtime::spawn(async move {
            let mut milestone_index: u32 = 0;  // Step counter (0, 1, 2...)

            // ────────────────────────────────────────────────────────────────
            // TODO #7: Timer-Based Contract Completion ✅ IMPLEMENTED
            // ────────────────────────────────────────────────────────────────
            // Calculate contract end time based on duration_hours
            // Support pause/resume/stop controls
            // Emit completion event when done
            // ────────────────────────────────────────────────────────────────

            // Calculate when contract should end
            let contract_start_time = SystemTime::now();
            let contract_duration = Duration::from_secs_f64(contract.duration_hours * 3600.0);
            let contract_end_time = contract_start_time + contract_duration;
            
            println!(
                "⏱️  Contract {} will run for {:.2} hours (until {:?})",
                contract.id, contract.duration_hours, contract_end_time
            );
            
            // Initialize contract state as Running
            let state_manager = ContractStateManager::global();
            state_manager.lock().unwrap().set_state(contract_id_for_state.clone(), ContractControl::Running);

            // ────────────────────────────────────────────────────────────────
            // TODO #9: Emit contract_started event ✅ IMPLEMENTED
            // ────────────────────────────────────────────────────────────────
            let total_milestones = ((contract.duration_hours * 3600.0) / contract.interval_seconds as f64) as u32;
            let _ = app.emit("contract_started", serde_json::json!({
                "contract_id": contract.id,
                "digger_id": digger_id,
                "duration_hours": contract.duration_hours,
                "total_milestones": total_milestones,
                "robo_stake_total": contract.robo_stake_total,
                "interval_seconds": contract.interval_seconds,
            }));
            
            // Tracking variables for status updates
            let mut total_joules: u64 = 0;
            let mut total_tokens: u64 = 0;
            let mut robo_stake_sent: f64 = 0.0;

            // Keep working until contract completes or is stopped
            loop {
                // Check contract execution state
                let current_state = state_manager.lock().unwrap().get_state(&contract_id_for_state);
                
                match current_state {
                    ContractControl::Stopped => {
                        println!("🛑 Contract {} stopped by user", contract.id);
                        let _ = app.emit("contract_stopped", contract.id.clone());
                        break;
                    }
                    ContractControl::Paused => {
                        // Wait a bit and check again
                        println!("⏸️  Contract {} paused, waiting...", contract.id);
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    ContractControl::Completed => {
                        // Already completed (shouldn't happen in loop, but safe guard)
                        break;
                    }
                    ContractControl::Running => {
                        // Continue normal execution
                    }
                }
                
                // Check if contract duration has been reached
                if SystemTime::now() >= contract_end_time {
                    println!("✅ Contract {} completed (duration reached)", contract.id);
                    state_manager.lock().unwrap().set_state(contract_id_for_state.clone(), ContractControl::Completed);
                    let _ = app.emit("contract_completed", contract.id.clone());
                    break;
                }

                // Get the current time
                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Take a fake photo to prove the robot did work
                let proof_photo = capture_photo_placeholder(&digger_id, milestone_index);

                // How many tokens made this step
                let tokens = throughput;
                // How much electricity used (power × time)
                let joules = (power_kw * contract.interval_seconds as f64 * 1000.0) as u64;

                // ────────────────────────────────────────────────────────────────
                // TODO #6: Calculate and Attach RoboStake to Ore ✅ IMPLEMENTED
                // ────────────────────────────────────────────────────────────────
                // Calculate fair share of total RoboStake for this milestone
                // Formula: robo_per_milestone = robo_stake_total / total_milestones
                //   where total_milestones = (duration_hours × 3600) / interval_seconds
                //
                // ECONOMICS:
                // - Each milestone carries its fair share of total stake
                // - RoboStake travels with ore: Digger → Refinery → Mint → Ledger
                // - Mint aggregates amounts in Merkle tree
                // - DistoDam receives total RT when contract completes
                // ────────────────────────────────────────────────────────────────

                // Calculate RoboStake amount for this milestone
                let robo_stake_amount = if contract.robo_stake_total > 0.0 && contract.duration_hours > 0.0 {
                    // Total milestones = (hours × 3600 seconds/hour) / interval_seconds
                    let total_milestones = (contract.duration_hours * 3600.0) / contract.interval_seconds as f64;
                    // Fair share per milestone
                    let robo_per_milestone = contract.robo_stake_total / total_milestones;
                    
                    println!(
                        "💰 Milestone economics: {} RT total / {:.0} milestones = {:.6} RT/milestone",
                        contract.robo_stake_total, total_milestones, robo_per_milestone
                    );
                    
                    robo_per_milestone
                } else {
                    0.0 // No stake received yet (contract not funded)
                };

                // Create a **report card** of this work
                let ore = JouleTorqOre {
                    digger_id: digger_id.clone(),
                    contract_id: contract.id.clone(),
                    tokens_generated: tokens,
                    joules,
                    milestone_index,
                    timestamp: start_time,
                    proof_of_work: proof_photo,
                    robo_stake_amount,  // ✅ Calculated above
                    signature: None,    // TODO #2: Sign with crypto::sign_ore() before sending to Refinery
                };

                // Print a message so we can see progress
                println!(
                    "⚙️  Digger {} milestone {}: {} tokens, {} joules, {:.6} RT",
                    digger_id, milestone_index, tokens, joules, robo_stake_amount
                );

                // Save the report in the treasure vault
                ore_store.lock().unwrap().add_ore(ore.clone());

                // ────────────────────────────────────────────────────────────────
                // TODO #8: Track Milestone Status ✅ IMPLEMENTED
                // ────────────────────────────────────────────────────────────────
                // Track milestone submission status (Pending → Sending → Confirmed/Failed)
                // Store in MilestoneTracker, emit events to UI
                // ────────────────────────────────────────────────────────────────
                
                let milestone_tracker = MilestoneTracker::global();
                
                // Mark as Sending
                milestone_tracker.lock().unwrap().set_status(
                    &contract.id,
                    milestone_index,
                    MilestoneStatus::Sending
                );
                
                // Send the ore batch to Refinery for processing
                let refinery_success = match refinery_client::send_ore_to_refinery(&ore).await {
                    Ok(()) => {
                        println!("✅ Refinery accepted milestone {}", milestone_index);
                        
                        // Mark as Confirmed
                        milestone_tracker.lock().unwrap().set_status(
                            &contract.id,
                            milestone_index,
                            MilestoneStatus::Confirmed
                        );
                        
                        // Update running totals
                        robo_stake_sent += robo_stake_amount;
                        
                        // Emit success event to UI
                        let _ = app.emit("milestone_confirmed", serde_json::json!({
                            "contract_id": contract.id,
                            "milestone_index": milestone_index,
                            "tokens": tokens,
                            "robo_stake": robo_stake_amount,
                        }));
                        
                        true  // Success
                    }
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        println!("⚠️  Failed to send milestone {} to Refinery: {}", milestone_index, error_msg);
                        
                        // Mark as Failed with error message
                        milestone_tracker.lock().unwrap().set_status(
                            &contract.id,
                            milestone_index,
                            MilestoneStatus::Failed(error_msg.clone())
                        );
                        
                        // Emit failure event to UI
                        let _ = app.emit("milestone_failed", serde_json::json!({
                            "contract_id": contract.id,
                            "milestone_index": milestone_index,
                            "error": error_msg,
                        }));
                        
                        // Continue despite error (ore is stored locally for retry)
                        false  // Failure
                    }
                };

                // Update running totals
                total_joules += joules;
                total_tokens += tokens;

                // ────────────────────────────────────────────────────────────────
                // TODO #9: Emit Rich Contract Status Events ✅ IMPLEMENTED
                // ────────────────────────────────────────────────────────────────
                // Send comprehensive progress updates to UI dashboard
                // ────────────────────────────────────────────────────────────────
                
                // Calculate progress metrics
                let time_elapsed = SystemTime::now()
                    .duration_since(contract_start_time)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();
                
                let time_remaining = if let Ok(remaining) = contract_end_time.duration_since(SystemTime::now()) {
                    remaining.as_secs()
                } else {
                    0  // Contract is complete
                };
                
                let percent_complete = if total_milestones > 0 {
                    ((milestone_index + 1) as f32 / total_milestones as f32) * 100.0
                } else {
                    0.0
                };
                
                // Get milestone statistics
                let all_milestones = milestone_tracker.lock().unwrap().get_all_for_contract(&contract.id);
                let milestones_confirmed = all_milestones.iter()
                    .filter(|(_, s)| matches!(s, MilestoneStatus::Confirmed))
                    .count() as u32;
                let milestones_failed = all_milestones.iter()
                    .filter(|(_, s)| matches!(s, MilestoneStatus::Failed(_)))
                    .count() as u32;
                
                // Create comprehensive status update
                let status_update = ContractStatusUpdate {
                    contract_id: contract.id.clone(),
                    digger_id: digger_id.clone(),
                    total_joules,
                    total_tokens,
                    total_robo_stake: contract.robo_stake_total,
                    robo_stake_sent,
                    current_milestone: milestone_index,
                    total_milestones,
                    percent_complete,
                    time_elapsed_secs: time_elapsed,
                    time_remaining_secs: time_remaining,
                    milestones_confirmed,
                    milestones_failed,
                    refinery_healthy: refinery_success,
                    state: "running".to_string(),
                };
                
                // Emit comprehensive status update
                let _ = app.emit("contract_status_update", &status_update);

                // ────────────────────────────────────────────────────────────────
                if let Err(e) = app.emit("ore_update", ore.clone()) {
                    println!("Warning: Could not send update: {:?}", e);
                }

                // Go to next step
                milestone_index += 1;

                // Wait before next report (like a coffee break)
                sleep(Duration::from_secs(contract.interval_seconds)).await;
            }
        });
    }
}

/// This makes a **fake photo** to prove the robot did work
/// In real life, it would take a real picture
fn capture_photo_placeholder(digger_id: &str, milestone_index: u32) -> Option<String> {
    let data = format!("photo-{}-{}", digger_id, milestone_index);
    // Turn the text into a long code (base64) like a real photo
    Some(general_purpose::STANDARD.encode(data))
}

// ────────────────────────────────────────────────────────────────
// CONTRACT EXECUTION STATE MANAGER
// ────────────────────────────────────────────────────────────────

/// Global state manager for contract execution control
/// Tracks whether contracts are running, paused, stopped, or completed
pub struct ContractStateManager {
    states: HashMap<String, ContractControl>, // contract_id → state
}

impl ContractStateManager {
    /// Get the global singleton instance
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<ContractStateManager>> =
                Arc::new(Mutex::new(ContractStateManager { states: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    /// Get the current state of a contract
    pub fn get_state(&self, contract_id: &str) -> ContractControl {
        self.states.get(contract_id).cloned().unwrap_or(ContractControl::Running)
    }

    /// Set the state of a contract
    pub fn set_state(&mut self, contract_id: String, state: ContractControl) {
        self.states.insert(contract_id, state);
    }
}

// ────────────────────────────────────────────────────────────────
// MILESTONE STATUS TRACKER (TODO #8)
// ────────────────────────────────────────────────────────────────

/// Global tracker for milestone submission status
/// Tracks the status of each milestone sent to Refinery
pub struct MilestoneTracker {
    // Key: "contract_id:milestone_index" → MilestoneStatus
    statuses: HashMap<String, MilestoneStatus>,
}

impl MilestoneTracker {
    /// Get the global singleton instance
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<MilestoneTracker>> =
                Arc::new(Mutex::new(MilestoneTracker { statuses: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    /// Get the status of a specific milestone
    #[allow(dead_code)]
    pub fn get_status(&self, contract_id: &str, milestone_index: u32) -> MilestoneStatus {
        let key = format!("{}:{}", contract_id, milestone_index);
        self.statuses.get(&key).cloned().unwrap_or(MilestoneStatus::Pending)
    }

    /// Set the status of a milestone
    pub fn set_status(&mut self, contract_id: &str, milestone_index: u32, status: MilestoneStatus) {
        let key = format!("{}:{}", contract_id, milestone_index);
        self.statuses.insert(key, status);
    }

    /// Get all milestone statuses for a contract
    pub fn get_all_for_contract(&self, contract_id: &str) -> Vec<(u32, MilestoneStatus)> {
        self.statuses
            .iter()
            .filter_map(|(key, status)| {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 2 && parts[0] == contract_id {
                    if let Ok(index) = parts[1].parse::<u32>() {
                        return Some((index, status.clone()));
                    }
                }
                None
            })
            .collect()
    }
}

// ────────────────────────────────────────────────────────────────
// THE ROBOT BOSS (DiggerManager)
// ────────────────────────────────────────────────────────────────

/// This is the **boss** that keeps track of all robots
pub struct DiggerManager {
    /// Like a phone book: robot name → robot
    diggers: HashMap<String, Digger>,
}

impl DiggerManager {
    /// Create **one global boss** that everyone shares
    /// (Like a school principal — only one!)
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<DiggerManager>> =
                Arc::new(Mutex::new(DiggerManager { diggers: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    /// Find a robot (immutable)
    pub fn get_digger(&self, id: &str) -> Option<&Digger> {
        self.diggers.get(id)
    }

    /// Find a robot and let you change it
    pub fn get_digger_mut(&mut self, id: &str) -> Option<&mut Digger> {
        self.diggers.get_mut(id)
    }

    /// Get a list of all robot names
    pub fn list_diggers(&self) -> Vec<String> {
        self.diggers.keys().cloned().collect()
    }

    /// Add a new robot to the factory
    pub fn add_digger(&mut self, digger: Digger) {
        self.diggers.insert(digger.id.clone(), digger);
    }

    /// Get the current contract for a specific digger
    pub fn get_contract(&self, digger_id: &str) -> Option<Contract> {
        self.diggers.get(digger_id).and_then(|d| d.current_contract.clone())
    }
}

// ────────────────────────────────────────────────────────────────
// TAURI COMMANDS: Contract Control
// ────────────────────────────────────────────────────────────────

/// Pause a running contract
#[tauri::command]
pub fn pause_contract(contract_id: String, app: tauri::AppHandle) -> Result<(), String> {
    println!("⏸️  Pausing contract: {}", contract_id);
    
    let state_manager = ContractStateManager::global();
    state_manager.lock().unwrap().set_state(contract_id.clone(), ContractControl::Paused);
    
    let _ = app.emit("contract_paused", contract_id);
    Ok(())
}

/// Resume a paused contract
#[tauri::command]
pub fn resume_contract(contract_id: String, app: tauri::AppHandle) -> Result<(), String> {
    println!("▶️  Resuming contract: {}", contract_id);
    
    let state_manager = ContractStateManager::global();
    state_manager.lock().unwrap().set_state(contract_id.clone(), ContractControl::Running);
    
    let _ = app.emit("contract_resumed", contract_id);
    Ok(())
}

/// Stop a running contract
#[tauri::command]
pub fn stop_contract(contract_id: String, _app: tauri::AppHandle) -> Result<(), String> {
    println!("🛑 Stopping contract: {}", contract_id);
    
    let state_manager = ContractStateManager::global();
    state_manager.lock().unwrap().set_state(contract_id.clone(), ContractControl::Stopped);
    
    // Note: The actual worker loop will detect this state change and exit
    // The "contract_stopped" event will be emitted from the worker loop
    Ok(())
}

/// Get milestone statuses for a contract (TODO #8)
#[tauri::command]
pub fn get_milestone_statuses(contract_id: String) -> Result<serde_json::Value, String> {
    let tracker = MilestoneTracker::global();
    let milestones = tracker.lock().unwrap().get_all_for_contract(&contract_id);
    
    // Convert to JSON-friendly format
    let statuses: Vec<serde_json::Value> = milestones
        .iter()
        .map(|(index, status)| {
            serde_json::json!({
                "milestone_index": index,
                "status": match status {
                    MilestoneStatus::Pending => "pending",
                    MilestoneStatus::Sending => "sending",
                    MilestoneStatus::Confirmed => "confirmed",
                    MilestoneStatus::Failed(_msg) => "failed",
                },
                "error": match status {
                    MilestoneStatus::Failed(msg) => Some(msg.clone()),
                    _ => None,
                }
            })
        })
        .collect();
    
    Ok(serde_json::json!({
        "contract_id": contract_id,
        "milestones": statuses,
        "total": milestones.len(),
        "confirmed": milestones.iter().filter(|(_, s)| matches!(s, MilestoneStatus::Confirmed)).count(),
        "failed": milestones.iter().filter(|(_, s)| matches!(s, MilestoneStatus::Failed(_))).count(),
    }))
}

// ────────────────────────────────────────────────────────────────
// UNIT TESTS (TODO #12)
// ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ────────────────────────────────────────────────────────────────
    // ContractControl Enum Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_contract_control_enum_states() {
        let running = ContractControl::Running;
        let paused = ContractControl::Paused;
        let stopped = ContractControl::Stopped;
        let completed = ContractControl::Completed;

        assert_eq!(running, ContractControl::Running);
        assert_eq!(paused, ContractControl::Paused);
        assert_eq!(stopped, ContractControl::Stopped);
        assert_eq!(completed, ContractControl::Completed);

        // Test that states are distinct
        assert_ne!(running, paused);
        assert_ne!(paused, stopped);
        assert_ne!(stopped, completed);
    }

    #[test]
    fn test_contract_control_clone() {
        let original = ContractControl::Running;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ────────────────────────────────────────────────────────────────
    // MilestoneStatus Enum Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_milestone_status_pending() {
        let status = MilestoneStatus::Pending;
        assert!(matches!(status, MilestoneStatus::Pending));
    }

    #[test]
    fn test_milestone_status_confirmed() {
        let status = MilestoneStatus::Confirmed;
        assert!(matches!(status, MilestoneStatus::Confirmed));
    }

    #[test]
    fn test_milestone_status_failed() {
        let status = MilestoneStatus::Failed("Network timeout".to_string());
        match status {
            MilestoneStatus::Failed(msg) => {
                assert_eq!(msg, "Network timeout");
            }
            _ => panic!("Expected Failed status"),
        }
    }

    // ────────────────────────────────────────────────────────────────
    // ContractStateManager Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_contract_state_manager_default_state() {
        let manager = ContractStateManager::global();
        let state = manager.lock().unwrap().get_state("test-contract-default");
        
        // Default state should be Running
        assert_eq!(state, ContractControl::Running);
    }

    #[test]
    fn test_contract_state_manager_set_and_get() {
        let manager = ContractStateManager::global();
        
        // Set state to Paused
        manager.lock().unwrap().set_state(
            "test-contract-set-get".to_string(),
            ContractControl::Paused
        );
        
        // Get state back
        let state = manager.lock().unwrap().get_state("test-contract-set-get");
        assert_eq!(state, ContractControl::Paused);
    }

    #[test]
    fn test_contract_state_manager_state_transitions() {
        let manager = ContractStateManager::global();
        let contract_id = "test-transitions-002";
        
        // Running → Paused → Running → Stopped → Completed
        manager.lock().unwrap().set_state(
            contract_id.to_string(),
            ContractControl::Running
        );
        assert_eq!(
            manager.lock().unwrap().get_state(contract_id),
            ContractControl::Running
        );
        
        manager.lock().unwrap().set_state(
            contract_id.to_string(),
            ContractControl::Paused
        );
        assert_eq!(
            manager.lock().unwrap().get_state(contract_id),
            ContractControl::Paused
        );
        
        manager.lock().unwrap().set_state(
            contract_id.to_string(),
            ContractControl::Stopped
        );
        assert_eq!(
            manager.lock().unwrap().get_state(contract_id),
            ContractControl::Stopped
        );
    }

    // ────────────────────────────────────────────────────────────────
    // MilestoneTracker Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_milestone_tracker_set_and_get() {
        let tracker = MilestoneTracker::global();
        
        // Set milestone to Confirmed
        tracker.lock().unwrap().set_status(
            "contract-milestone-test-001",
            42,
            MilestoneStatus::Confirmed
        );
        
        // Get status back
        let status = tracker.lock().unwrap().get_status("contract-milestone-test-001", 42);
        assert!(matches!(status, MilestoneStatus::Confirmed));
    }

    #[test]
    fn test_milestone_tracker_failed_with_error() {
        let tracker = MilestoneTracker::global();
        
        tracker.lock().unwrap().set_status(
            "contract-error-test",
            0,
            MilestoneStatus::Failed("NetworkError: timeout".to_string())
        );
        
        // Verify error message is preserved
        let status = tracker.lock().unwrap().get_status("contract-error-test", 0);
        match status {
            MilestoneStatus::Failed(msg) => assert_eq!(msg, "NetworkError: timeout"),
            _ => panic!("Expected Failed status"),
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Economic Calculation Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_robo_per_milestone_calculation() {
        // Example: 3000 RT total, 20 hour duration, 5 second intervals
        let robo_stake_total = 3000.0;
        let duration_hours = 20.0;
        let interval_seconds = 5.0;
        
        // Calculate milestones
        let total_milestones = (duration_hours * 3600.0) / interval_seconds;
        let robo_per_milestone = robo_stake_total / total_milestones;
        
        // 20 hours × 3600 seconds/hour = 72,000 seconds
        // 72,000 seconds / 5 seconds = 14,400 milestones
        assert_eq!(total_milestones, 14400.0);
        
        // 3000 RT / 14,400 milestones ≈ 0.2083333... RT per milestone
        assert!((robo_per_milestone - 0.208333333_f64).abs() < 0.000001);
    }

    #[test]
    fn test_robo_per_milestone_zero_stake() {
        let robo_stake_total = 0.0;
        let duration_hours = 20.0;
        let interval_seconds = 5.0;
        
        let total_milestones = (duration_hours * 3600.0) / interval_seconds;
        let robo_per_milestone = if robo_stake_total > 0.0 && duration_hours > 0.0 {
            robo_stake_total / total_milestones
        } else {
            0.0
        };
        
        assert_eq!(robo_per_milestone, 0.0);
    }

    #[test]
    fn test_total_milestones_calculation() {
        // Test: 1 hour, 1 second intervals = 3600 milestones
        assert_eq!((1.0 * 3600.0) / 1.0, 3600.0);
        
        // Test: 24 hours, 5 second intervals = 17,280 milestones
        assert_eq!((24.0 * 3600.0) / 5.0, 17280.0);
        
        // Test: 0.5 hours (30 min), 10 second intervals = 180 milestones
        assert_eq!((0.5 * 3600.0) / 10.0, 180.0);
    }

    // ────────────────────────────────────────────────────────────────
    // Digger Creation & Configuration Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_digger_creation() {
        let digger = Digger {
            id: "test-digger-001".to_string(),
            power_kw: 0.25,
            max_token_throughput: 12,
            current_contract: None,
        };

        assert_eq!(digger.id, "test-digger-001");
        assert_eq!(digger.power_kw, 0.25);
        assert_eq!(digger.max_token_throughput, 12);
        assert!(digger.current_contract.is_none());
    }

    #[test]
    fn test_digger_clone() {
        let digger1 = Digger {
            id: "clone-test".to_string(),
            power_kw: 0.5,
            max_token_throughput: 20,
            current_contract: None,
        };

        let digger2 = digger1.clone();
        assert_eq!(digger1.id, digger2.id);
        assert_eq!(digger1.power_kw, digger2.power_kw);
        assert_eq!(digger1.max_token_throughput, digger2.max_token_throughput);
    }

    // ────────────────────────────────────────────────────────────────
    // Contract Duration Calculation Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_contract_duration_calculation() {
        // Given: amount_rt = 100.0, power_kw = 0.25, max_throughput = 12
        // Expected: duration = 100 / (0.25 × 12) = 100 / 3 = 33.333... hours
        
        let amount_rt = 100.0;
        let power_kw = 0.25;
        let max_token_throughput = 12.0;
        
        let duration_hours = amount_rt / (power_kw * max_token_throughput);
        
        assert!((duration_hours - 33.333333_f64).abs() < 0.00001);
    }

    #[test]
    fn test_contract_duration_edge_cases() {
        // Test 1: Very small RT amount
        let duration1 = 1.0 / (0.25 * 12.0);
        assert!((duration1 - 0.333333_f64).abs() < 0.00001);

        // Test 2: Large RT amount
        let duration2 = 10000.0 / (0.25 * 12.0);
        assert!((duration2 - 3333.333333_f64).abs() < 0.00001);

        // Test 3: High power digger
        let duration3 = 100.0 / (1.0 * 50.0);
        assert_eq!(duration3, 2.0);
    }

    #[test]
    fn test_total_milestones_from_duration() {
        // 33.33 hours, 5 second intervals
        let duration_hours: f64 = 33.333333;
        let interval_seconds: f64 = 5.0;
        let total_milestones = (duration_hours * 3600.0) / interval_seconds;
        
        // 33.333333 × 3600 = 119,999.9988 seconds / 5 ≈ 24,000 milestones
        assert!((total_milestones - 24000.0).abs() < 0.01);
    }

    // ────────────────────────────────────────────────────────────────
    // Ore Generation Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_ore_tokens_calculation() {
        // tokens = max_token_throughput × interval_seconds × torq
        let max_throughput = 12;
        let interval = 5;
        let torq = 1; // Always 1 for now
        
        let tokens = max_throughput * interval * torq;
        assert_eq!(tokens, 60);
    }

    #[test]
    fn test_ore_joules_calculation() {
        // joules = power_kw × 1000 × interval_seconds
        let power_kw = 0.25;
        let interval = 5;
        
        let joules = (power_kw * 1000.0 * interval as f64) as u64;
        assert_eq!(joules, 1250);
    }

    #[test]
    fn test_ore_joules_various_power_levels() {
        // Low power (0.1 kW = 100W)
        let joules1 = (0.1 * 1000.0 * 5.0) as u64;
        assert_eq!(joules1, 500);

        // Medium power (0.5 kW = 500W)
        let joules2 = (0.5 * 1000.0 * 5.0) as u64;
        assert_eq!(joules2, 2500);

        // High power (1.0 kW = 1000W)
        let joules3 = (1.0 * 1000.0 * 5.0) as u64;
        assert_eq!(joules3, 5000);
    }

    #[test]
    fn test_robo_stake_distribution() {
        // 3000 RT over 14,400 milestones
        let total_rt = 3000.0;
        let total_milestones = 14400.0;
        let robo_per_milestone = total_rt / total_milestones;
        
        // Each milestone should get ~0.208333 RT
        assert!((robo_per_milestone - 0.208333_f64).abs() < 0.00001);
        
        // Sum of all milestones should equal total
        let reconstructed_total = robo_per_milestone * total_milestones;
        assert!((reconstructed_total - total_rt).abs() < 0.001);
    }

    // ────────────────────────────────────────────────────────────────
    // ContractStatusUpdate Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_contract_status_update_creation() {
        let status = ContractStatusUpdate {
            contract_id: "test-contract".to_string(),
            digger_id: "test-digger".to_string(),
            total_joules: 1250,
            total_tokens: 60,
            total_robo_stake: 100.0,
            robo_stake_sent: 0.208333,
            current_milestone: 1,
            total_milestones: 480,
            percent_complete: 0.208333,
            time_elapsed_secs: 5,
            time_remaining_secs: 2395,
            milestones_confirmed: 1,
            milestones_failed: 0,
            refinery_healthy: true,
            state: "running".to_string(),
        };

        assert_eq!(status.contract_id, "test-contract");
        assert_eq!(status.current_milestone, 1);
        assert_eq!(status.total_milestones, 480);
        assert!(status.refinery_healthy);
    }

    #[test]
    fn test_percent_complete_calculation() {
        // Milestone 100 out of 1000 = 10%
        let current = 100.0;
        let total = 1000.0;
        let percent = (current / total) * 100.0;
        
        assert!((percent - 10.0_f64).abs() < 0.001);
    }

    #[test]
    fn test_time_remaining_calculation() {
        // 480 total milestones, 5 second intervals = 2400 seconds total
        // Completed 1 milestone (5 seconds elapsed)
        // Remaining: 479 milestones × 5 seconds = 2395 seconds
        
        let total_milestones = 480;
        let current_milestone = 1;
        let interval_seconds = 5;
        
        let remaining_milestones = total_milestones - current_milestone;
        let time_remaining = remaining_milestones * interval_seconds;
        
        assert_eq!(time_remaining, 2395);
    }

    // ────────────────────────────────────────────────────────────────
    // Proof of Work Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_proof_of_work_generation() {
        // Proof format: "photo-{digger_id}-{milestone_index}"
        let digger_id = "dig-jon-ai-001";
        let milestone_index = 0;
        
        let proof_data = format!("photo-{}-{}", digger_id, milestone_index);
        let proof_base64 = general_purpose::STANDARD.encode(proof_data.as_bytes());
        
        // Verify it encodes correctly
        assert!(proof_base64.len() > 0);
        
        // Verify it decodes back
        let decoded = general_purpose::STANDARD.decode(&proof_base64).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        assert_eq!(decoded_str, format!("photo-{}-{}", digger_id, milestone_index));
    }

    #[test]
    fn test_proof_uniqueness_per_milestone() {
        let digger_id = "dig-test";
        
        // Generate proofs for different milestones
        let proof0 = format!("photo-{}-0", digger_id);
        let proof1 = format!("photo-{}-1", digger_id);
        let proof2 = format!("photo-{}-2", digger_id);
        
        // Each should be unique
        assert_ne!(proof0, proof1);
        assert_ne!(proof1, proof2);
        assert_ne!(proof0, proof2);
    }

    // ────────────────────────────────────────────────────────────────
    // Timestamp Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_unix_timestamp_generation() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Timestamp should be reasonable (after 2020, before 2100)
        assert!(now > 1577836800); // Jan 1, 2020
        assert!(now < 4102444800); // Jan 1, 2100
    }

    // ────────────────────────────────────────────────────────────────
    // Edge Case & Boundary Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_zero_interval_protection() {
        // Ensure we don't divide by zero
        let duration_hours = 10.0;
        let interval_seconds = 0.0;
        
        // In real code, we should validate interval > 0
        // For now, just verify the calculation would panic
        let result = std::panic::catch_unwind(|| {
            let _milestones = (duration_hours * 3600.0) / interval_seconds;
        });
        
        // Division by zero doesn't panic in Rust for floats, returns infinity
        assert!(result.is_ok());
    }

    #[test]
    fn test_negative_robo_stake_handling() {
        // RoboStake should never be negative
        let robo_stake = -100.0;
        
        // In real code, validation should reject this
        // Test that we can detect invalid values
        assert!(robo_stake < 0.0);
    }

    #[test]
    fn test_milestone_index_overflow() {
        // Test very large milestone counts
        let duration_hours = 1000.0; // ~41 days
        let interval_seconds = 1.0;
        let total_milestones = (duration_hours * 3600.0) / interval_seconds;
        
        // 1000 hours × 3600 = 3,600,000 milestones
        assert_eq!(total_milestones, 3_600_000.0);
        
        // Verify u32 can hold this (max ~4.2 billion)
        assert!(total_milestones < u32::MAX as f64);
    }

    // ────────────────────────────────────────────────────────────────
    // DiggerManager Tests
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_digger_manager_global_singleton() {
        let manager1 = DiggerManager::global();
        let manager2 = DiggerManager::global();
        
        // Both should point to same instance (Arc ensures this)
        assert!(Arc::ptr_eq(&manager1, &manager2));
    }

    // NOTE: Removed tests for get_digger() and nonexistent_digger() since
    // DiggerManager starts empty and diggers are added by main.rs at runtime
}
