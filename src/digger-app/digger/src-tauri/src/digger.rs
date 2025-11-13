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

                // Create a **report card** of this work
                let ore = JouleTorqOre {
                    digger_id: digger_id.clone(),
                    contract_id: contract.id.clone(),
                    tokens_generated: tokens,
                    joules,
                    milestone_index,
                    timestamp: start_time,
                    proof_of_work: proof_photo,
                };

                // Print a message so we can see progress
                println!(
                    "Digger {} step {}: {} tokens, {} joules",
                    digger_id, milestone_index, tokens, joules
                );

                // Save the report in the treasure vault
                ore_store.lock().unwrap().add_ore(ore.clone());

                // Send a live update to the app window
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
}