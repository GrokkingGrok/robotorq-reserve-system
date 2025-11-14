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
                // Send Ore to Refinery ✅ IMPLEMENTED
                // ────────────────────────────────────────────────────────────────
                // Send the ore batch to Refinery for processing into ingots
                // Refinery will validate signature (TODO #2) and create TokenTorqIngot
                // ────────────────────────────────────────────────────────────────
                
                match refinery_client::send_ore_to_refinery(&ore) {
                    Ok(()) => {
                        println!("✅ Refinery accepted milestone {}", milestone_index);
                        // TODO #8: Mark milestone as Confirmed
                    }
                    Err(e) => {
                        println!("⚠️  Failed to send milestone {} to Refinery: {}", milestone_index, e);
                        // TODO #8: Mark milestone as Failed(error_message)
                        // For now, continue despite error (ore is stored locally)
                    }
                }

                // ────────────────────────────────────────────────────────────────
                // TODO #8: Track Milestone Submission Status
                // ────────────────────────────────────────────────────────────────
                // CURRENT: Just store locally, no Refinery feedback
                // NEEDED: Track whether ore was successfully sent and confirmed
                //
                // IMPLEMENTATION:
                // 1. Create MilestoneStatus enum:
                //    enum MilestoneStatus {
                //        Pending,      // Not yet sent
                //        Sent,         // Sent to Refinery, awaiting confirmation
                //        Confirmed,    // Refinery accepted (200 OK)
                //        Failed(String), // Error (invalid sig, network, queue full)
                //    }
                //
                // 2. Store status in Contract or separate tracking map:
                //    HashMap<(contract_id, milestone_index), MilestoneStatus>
                //
                // 3. After calling send_ore_to_refinery():
                //    match result {
                //        Ok(_) => set_status(Confirmed),
                //        Err(RefineryError::NetworkError(e)) => set_status(Failed(e)),
                //        Err(RefineryError::QueueFull) => set_status(Failed("queue_full")),
                //        ...
                //    }
                //
                // 4. Include status in UI events (TODO #9)
                //
                // OBSERVABILITY:
                // - Dashboard shows: "45/100 milestones confirmed"
                // - Failed milestones can be retried
                // - Helps debug Refinery integration issues
                // - Provides audit trail for economics
                // ────────────────────────────────────────────────────────────────

                // Send a live update to the app window
                // ────────────────────────────────────────────────────────────────
                // TODO #9: Emit Rich Contract Status Events
                // ────────────────────────────────────────────────────────────────
                // CURRENT: Only sends basic ore_update with ore data
                // NEEDED: Send comprehensive economics and progress data to UI
                //
                // IMPLEMENTATION:
                // 1. Create ContractStatusUpdate struct:
                //    #[derive(Serialize, Clone)]
                //    struct ContractStatusUpdate {
                //        contract_id: String,
                //        digger_id: String,
                //        // Economics
                //        total_joules: u64,
                //        total_tokens: u64,
                //        total_robo_stake: f64,
                //        robo_stake_sent: f64,      // Sum of all sent milestones
                //        // Progress
                //        current_milestone: u32,
                //        total_milestones: u32,
                //        percent_complete: f32,
                //        time_elapsed_secs: u64,
                //        time_remaining_secs: u64,
                //        // Status
                //        milestones_sent: u32,
                //        milestones_confirmed: u32,
                //        milestones_failed: u32,
                //        refinery_healthy: bool,    // Last send succeeded?
                //    }
                //
                // 2. Calculate all fields using contract data
                // 3. Emit "contract_status_update" event (in addition to "ore_update")
                // 4. UI subscribes to this event for dashboard updates (TODO #10)
                //
                // UI DISPLAY (TODO #10):
                // - Economics panel: Total JouleTorq, RoboStake received, RoboStake sent
                // - Progress bar: current_milestone / total_milestones
                // - Timer: "18.5 hours remaining"
                // - Health indicator: Green (refinery_healthy=true), Red (false)
                // - Milestone status: "68/100 confirmed, 2 failed"
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
