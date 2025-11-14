// This tells Windows to hide the black console window when running the app
// (Only works when we're not in debug mode)
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

// These are like "import" statements — they bring in other parts of our code
mod digger;        // Controls the robots (Diggers)
mod ore_storage;   // Stores the digital gold (Ore) the robots make
mod contracts;     // Manages the job agreements (Contracts)
mod types;         // Defines the shapes of our data (like blueprints)
mod http_api;      // HTTP API for external services to query/control Digger

// We use these tools to manage our robots, jobs, and treasure
use digger::DiggerManager;        // The boss of all robots
use contracts::ContractManager;   // The boss of all job contracts
use ore_storage::OreStorage;      // The vault where Ore is kept

/// This is like the "Start Job" button in the app
/// It tells a robot to begin working on a contract
#[tauri::command]
fn start_contract(digger_id: String, contract_id: String, app: tauri::AppHandle) -> Result<String, String> {
    // Print a message so we can see what's happening
    println!("Start button pressed! Robot: {}, Job: {}", digger_id, contract_id);

    // Get the big bosses (managers) that control everything
    let manager = DiggerManager::global();        // Robot boss
    let contract_mgr = ContractManager::global(); // Job boss
    let ore_store = OreStorage::global();         // Treasure vault

    // Lock the robot boss so we can look inside safely
    let mut manager_lock = manager.lock().unwrap();
    // Find the robot by its name
    let manager_ref = match manager_lock.get_digger_mut(&digger_id) {
        Some(d) => d,  // Found it!
        None => {
            // Robot doesn't exist
            println!("Robot {} not found!", digger_id);
            return Err(format!("Robot {} not found", digger_id));
        }
    };

    // Lock the job boss to check the contract
    let contract = {
        let contract_mgr_lock = contract_mgr.lock().unwrap();
        match contract_mgr_lock.get_contract(&contract_id) {
            Some(c) => c,  // Job exists!
            None => {
                // Job doesn't exist
                println!("Job {} not found!", contract_id);
                return Err(format!("Job {} not found", contract_id));
            }
        }
    };

    // Make sure the job is allowed to start
    if !contract.authorized {
        println!("Job {} is not approved yet!", contract_id);
        return Err("Job not approved yet".into());
    }

    // Create a unique name for this work session
    let job_id = format!("{}-{}", digger_id, contract_id);
    println!("Starting job {} with robot {} → Session ID: {}", contract_id, digger_id, job_id);

    // Tell the robot to start working!
    // We give it the job details, treasure vault, app handle, and session ID
    manager_ref.start_contract(contract, ore_store, app.clone(), job_id);

    // Tell the app everything is good
    Ok(format!("Robot_{}_started_job_{}", digger_id, contract_id))
}

/// This is like the "Show All Robots" button
/// It lists every robot in the system
#[tauri::command]
fn list_diggers() -> Vec<String> {
    let mgr = DiggerManager::global();     // Get robot boss
    let lock = mgr.lock().unwrap();        // Open the robot list
    lock.list_diggers()                    // Return all robot names
}

/// This is like the "Show All Jobs" button
/// It lists every job contract in the system
#[tauri::command]
fn list_contracts() -> Vec<String> {
    let mgr = ContractManager::global();   // Get job boss
    let lock = mgr.lock().unwrap();        // Open the job list
    lock.list_contracts()                  // Return all job IDs
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
        // Add a robot to the system
        let diggers = DiggerManager::global();  // Robot boss
        let mut d = diggers.lock().unwrap();    // Open robot storage
        d.add_digger(digger::Digger {
            id: "dig-jon-ai-001".to_string(),   // Robot name
            power_kw: 0.25,                     // Uses 250 watts
            max_token_throughput: 12,           // Can make 12 coins/sec
            current_contract: None,             // Not working yet
        });
    }

    {
        // Add a job contract to the system
        let contracts = ContractManager::global();  // Job boss
        let mut c = contracts.lock().unwrap();      // Open job storage
        c.add_contract(types::Contract {
            id: "contract-001".to_string(),         // Job number
            authorized: true,                       // Approved to start
            torq: 5,                                // Work is worth 5× normal
            max_token_throughput: 12,               // Robot can do 12/sec
            interval_seconds: 5,                    // Send update every 5 sec
            total_tokens: 0,                        // No coins earned yet
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
            start_contract,    // "Start Job" button
            list_diggers,      // "Show Robots" button
            list_contracts     // "Show Jobs" button
        ])
        // Launch the app window
        .run(tauri::generate_context!())
        // Stop if something goes wrong
        .expect("Error: App failed to start");
}