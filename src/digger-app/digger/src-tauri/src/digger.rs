// This file controls the **robots (Diggers)** and how they work on jobs

use crate::types::{Contract, JouleTorqOre};  // Import our data blueprints
use crate::ore_storage::OreStorage;          // Import the treasure vault
use std::sync::{Arc, Mutex};                 // Tools to safely share data
use tokio::time::{sleep, Duration};          // Tool to wait between steps
use std::time::{SystemTime, UNIX_EPOCH};     // Tool to get the current time
use lazy_static::lazy_static;                // Magic to create one global thing
use std::collections::HashMap;               // Like a phone book: name → robot
use base64::{engine::general_purpose, Engine as _}; // Turn photos into text
use tauri::Emitter;                          // Send messages to the app window

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
        job_id: String,               // Unique name for this work session
    ) {
        // Save the job so the robot remembers what it's doing
        self.current_contract = Some(contract.clone());

        // Copy robot info so we can use it inside the background task
        let digger_id = self.id.clone();
        let power_kw = self.power_kw;
        // How many tokens this robot makes **per update**
        // = robot speed × job value
        let throughput = self.max_token_throughput * contract.torq as u64;

        // Start a background worker (like a factory machine)
        tauri::async_runtime::spawn(async move {
            let mut milestone_index: u32 = 0;  // Step counter (0, 1, 2...)

            // ────────────────────────────────────────────────────────────────
            // TODO #7: Timer-Based Contract Completion
            // ────────────────────────────────────────────────────────────────
            // CURRENT: Infinite loop - contract never ends!
            // NEEDED: Exit after contract.duration_hours (from TODO #3)
            //
            // IMPLEMENTATION:
            // 1. Get contract start time (SystemTime::now())
            // 2. Calculate end_time = start_time + (duration_hours × 3600 secs)
            // 3. Change `loop {}` to `while SystemTime::now() < end_time {}`
            // 4. After loop exits: emit "ore_complete" event to UI
            // 5. Clean up contract in ContractManager
            //
            // EXAMPLE:
            // - duration_hours = 20.0 (from TODO #3 calculation)
            // - start_time = Unix timestamp at contract start
            // - end_time = start_time + (20.0 × 3600) seconds
            // - Loop until SystemTime reaches end_time
            //
            // CONTRACT BEHAVIOR:
            // - Runs for FIXED DURATION (time-based, not work-based)
            // - Even if work finishes early, contract runs full duration
            // - Ensures predictable RoboStake compensation
            // ────────────────────────────────────────────────────────────────

            // Keep working forever (until stopped)
            loop {
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
                // TODO #6: Calculate and Attach RoboStake to Ore
                // ────────────────────────────────────────────────────────────────
                // CURRENT: Ore has no robo_stake_amount (economics lost!)
                // NEEDED: Calculate fair share of total stake for this milestone
                //
                // IMPLEMENTATION:
                // 1. Get contract.robo_stake_total (from TODO #3)
                // 2. Calculate total_milestones = (duration_hours × 3600) / interval_seconds
                // 3. Calculate robo_per_milestone = robo_stake_total / total_milestones
                // 4. Attach to ore: ore.robo_stake_amount = robo_per_milestone
                // 5. Sign ore using crypto::sign_ore() (TODO #2)
                // 6. Call refinery_client::send_ore_to_refinery(&ore) (TODO #4)
                // 7. Update milestone status based on result (TODO #8)
                //
                // EXAMPLE:
                // - robo_stake_total = 3000 RT (from /stake)
                // - duration_hours = 20.0
                // - interval_seconds = 1
                // - total_milestones = (20 × 3600) / 1 = 72000
                // - robo_per_milestone = 3000 / 72000 = 0.04166... RT
                //
                // ECONOMICS:
                // - Each milestone carries its fair share of total stake
                // - RoboStake travels with ore through pipeline:
                //   Digger → Refinery → TokenTorqIngot → Mint → Ledger
                // - Mint aggregates RoboStake amounts in Merkle tree
                // - DistoDam receives total RT when contract completes
                // ────────────────────────────────────────────────────────────────

                // Create a **report card** of this work
                let ore = JouleTorqOre {
                    digger_id: digger_id.clone(),
                    contract_id: contract.id.clone(),
                    tokens_generated: tokens,
                    joules,
                    milestone_index,
                    timestamp: start_time,
                    proof_of_work: proof_photo,
                    robo_stake_amount: 0.0,  // TODO #6: Calculate from contract.robo_stake_total / total_milestones
                    signature: None,          // TODO #2: Sign with crypto::sign_ore() before sending to Refinery
                };

                // Print a message so we can see progress
                println!(
                    "Digger {} step {}: {} tokens, {} joules",
                    digger_id, milestone_index, tokens, joules
                );

                // Save the report in the treasure vault
                ore_store.lock().unwrap().add_ore(ore.clone());

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

    /// Set/update the contract for a specific digger
    pub fn set_contract(&mut self, digger_id: &str, contract: Contract) -> Result<(), String> {
        match self.diggers.get_mut(digger_id) {
            Some(digger) => {
                digger.current_contract = Some(contract);
                Ok(())
            }
            None => Err(format!("Digger {} not found", digger_id))
        }
    }
}