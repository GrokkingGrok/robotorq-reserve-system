// This tells Windows to hide the black console window when running the app
// (Only works when we're not in debug mode)
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

// These are like "import" statements — they bring in other parts of our code
mod digger;        // Controls the robots (Diggers)
mod ore_storage;   // Stores the digital gold (Ore) the robots make
mod types;         // Defines the shapes of our data (like blueprints)
mod http_api;      // HTTP API for external services to query/control Digger

// We use these tools to manage our robots, jobs, and treasure
use digger::DiggerManager;        // The boss of all robots
use ore_storage::OreStorage;      // The vault where Ore is kept

/// This is like the "Start Job" button in the app
/// It tells a robot to begin working on a contract
#[tauri::command]
fn start_contract(digger_id: String, contract_id: String, app: tauri::AppHandle) -> Result<String, String> {
    // Print a message so we can see what's happening
    println!("Start button pressed! Robot: {}, Contract: {}", digger_id, contract_id);

    // Get the robot manager and treasure vault
    let manager = DiggerManager::global();        // Robot boss
    let ore_store = OreStorage::global();         // Treasure vault

    // Lock the robot boss so we can look inside safely
    let mut manager_lock = manager.lock().unwrap();
    
    // Get the contract from the digger
    let contract = match manager_lock.get_contract(&digger_id) {
        Some(c) if c.id == contract_id => c,  // Found matching contract!
        Some(_) => {
            println!("Contract ID mismatch for robot {}", digger_id);
            return Err(format!("Contract {} not found for robot {}", contract_id, digger_id));
        }
        None => {
            println!("No contract found for robot {}", digger_id);
            return Err(format!("No contract for robot {}", digger_id));
        }
    };

    // Make sure the job is allowed to start
    if !contract.authorized {
        println!("Contract {} is not approved yet!", contract_id);
        return Err("Contract not approved yet".into());
    }

    // Find the robot by its name
    let digger = match manager_lock.get_digger_mut(&digger_id) {
        Some(d) => d,  // Found it!
        None => {
            // Robot doesn't exist
            println!("Robot {} not found!", digger_id);
            return Err(format!("Robot {} not found", digger_id));
        }
    };

    // Create a unique name for this work session
    let job_id = format!("{}-{}", digger_id, contract_id);
    println!("Starting contract {} with robot {} → Session ID: {}", contract_id, digger_id, job_id);

    // Tell the robot to start working!
    // We give it the job details, treasure vault, app handle, and session ID
    digger.start_contract(contract, ore_store, app.clone(), job_id);

    // Tell the app everything is good
    Ok(format!("Robot_{}_started_contract_{}", digger_id, contract_id))
}

/// This is like the "Show All Robots" button
/// It lists every robot in the system
#[tauri::command]
fn list_diggers() -> Vec<String> {
    let mgr = DiggerManager::global();     // Get robot boss
    let lock = mgr.lock().unwrap();        // Open the robot list
    lock.list_diggers()                    // Return all robot names
}

/// This is like the "Show Current Contract" button
/// It returns the contract for a specific digger
#[tauri::command]
fn get_contract(digger_id: String) -> Result<String, String> {
    let mgr = DiggerManager::global();     // Get robot boss
    let lock = mgr.lock().unwrap();        // Open the robot list
    match lock.get_contract(&digger_id) {
        Some(contract) => Ok(contract.id),
        None => Err(format!("No active contract for digger {}", digger_id))
    }
}

/// This is the **main function** — where the program starts
fn main() {
    // Turn on helpful messages in the console
    env_logger::init();

    // ──────────────────────────────
    // FAKE DATA FOR TESTING
    // (Like setting up a demo world)
    // ──────────────────────────────

    {
        // Add a robot to the system with an initial contract
        let diggers = DiggerManager::global();  // Robot boss
        let mut d = diggers.lock().unwrap();    // Open robot storage
        
        // Create the initial contract
        let initial_contract = types::Contract {
            id: "contract-001".to_string(),         // Job number
            authorized: true,                       // Approved to start
            torq: 5,                                // Work is worth 5× normal
            max_token_throughput: 12,               // Robot can do 12/sec
            interval_seconds: 5,                    // Send update every 5 sec
            total_tokens: 0,                        // No coins earned yet
            robo_stake_total: 0.0,                  // No stake yet (set by POST /stake)
            duration_hours: 0.0,                    // Duration calculated when stake received
        };
        
        d.add_digger(digger::Digger {
            id: "dig-jon-ai-001".to_string(),       // Robot name
            power_kw: 0.25,                         // Uses 250 watts
            max_token_throughput: 12,               // Can make 12 coins/sec
            current_contract: Some(initial_contract), // Start with test contract
        });
    }

    // ──────────────────────────────
    // START THE APP
    // ──────────────────────────────

    // Start HTTP API server for external queries
    http_api::start_http_server();

    tauri::Builder::default()
        // Connect the buttons to the functions
        .invoke_handler(tauri::generate_handler![
            start_contract,    // "Start Contract" button
            list_diggers,      // "Show Robots" button
            get_contract       // "Show Contract" button
        ])
        // Launch the app window
        .run(tauri::generate_context!())
        // Stop if something goes wrong
        .expect("Error: App failed to start");
}