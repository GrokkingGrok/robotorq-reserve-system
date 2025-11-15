// HTTP API for Digger - allows Trust to query status and send stake
//
// Endpoints:
// GET  /robot/status → Check if robot is available
// POST /stake        → Receive RoboStake for a contract
// GET  /health       → Health check

use crate::digger::DiggerManager;
use crate::types::Contract;
use serde::{Deserialize, Serialize};
use tiny_http::{Response, Server};
#[allow(unused_imports)] // False positive: Read trait used by request.as_reader().read_to_string()
use std::io::Read;
use std::sync::{Arc, Mutex};

// Type alias for contract starter function
// In GUI mode: calls digger.start_contract with AppHandle
// In headless mode: calls headless_executor::start_headless_contract
type ContractStarter = Arc<Mutex<Option<Box<dyn Fn(String, f64, u64, Contract) + Send + 'static>>>>;

// Global contract starter (set by main.rs or headless.rs)
lazy_static::lazy_static! {
    static ref CONTRACT_STARTER: ContractStarter = Arc::new(Mutex::new(None));
}

/// Set the contract starter function (called from main or headless binary)
pub fn set_contract_starter<F>(starter: F)
where
    F: Fn(String, f64, u64, Contract) + Send + 'static,
{
    *CONTRACT_STARTER.lock().unwrap() = Some(Box::new(starter));
}

/// Response for /robot/status
#[derive(Serialize)]
struct RobotStatusResponse {
    available: bool,
    digger_id: String,
    current_contract: Option<String>,
}

/// Request body for /stake
#[derive(Deserialize)]
struct StakeRequest {
    contract_id: String,
    amount_rt: f64,
    // Note: Trust may send 'builder' field, but we don't use it yet
}

/// Response for /stake
#[derive(Serialize)]
struct StakeResponse {
    status: String,
    contract_id: String,
    message: String,
}

/// Start the HTTP server on port 9000
pub fn start_http_server() {
    let handle = std::thread::spawn(|| {
        println!("🔧 [HTTP Thread] Thread started, attempting to bind HTTP server to 127.0.0.1:9000...");
        
        let server = match Server::http("127.0.0.1:9000") {
            Ok(s) => {
                println!("✅ [HTTP Thread] HTTP server successfully bound to 127.0.0.1:9000");
                s
            }
            Err(e) => {
                eprintln!("❌ [HTTP Thread] FATAL: Failed to bind HTTP server to 127.0.0.1:9000");
                eprintln!("   Error: {}", e);
                eprintln!("   Possible causes:");
                eprintln!("   - Port 9000 already in use by another process");
                eprintln!("   - Permission denied (try running as administrator)");
                eprintln!("   - Firewall blocking the port");
                panic!("Cannot start HTTP server: {}", e);
            }
        };
        
        println!("🌐 [HTTP Thread] Digger HTTP API now listening on 127.0.0.1:9000");
        println!("   [HTTP Thread] Ready to accept requests...");
        println!("   [HTTP Thread] Entering request loop...");

        for (idx, request) in server.incoming_requests().enumerate() {
            let url = request.url().to_string();
            println!("📥 [HTTP Thread] Request #{}: {} {}", idx + 1, request.method(), url);
            
            match (request.method().as_str(), url.as_str()) {
                ("GET", "/robot/status") => {
                    println!("   [HTTP Thread] Handling /robot/status");
                    handle_robot_status(request)
                }
                ("POST", "/stake") => {
                    println!("   [HTTP Thread] Handling /stake");
                    handle_stake(request)
                }
                ("GET", "/health") => {
                    println!("   [HTTP Thread] Handling /health");
                    let response = Response::from_string("ok");
                    let _ = request.respond(response);
                }
                _ => {
                    println!("   [HTTP Thread] 404 Not Found");
                    let response = Response::from_string("Not Found").with_status_code(404);
                    let _ = request.respond(response);
                }
            }
        }
        
        println!("⚠️ [HTTP Thread] Request loop exited (this should never happen!)");
    });
    
    println!("📤 [Main Thread] HTTP server thread spawned, handle: {:?}", handle.thread().id());
}

fn handle_robot_status(request: tiny_http::Request) {
    let manager = DiggerManager::global();
    let lock = manager.lock().unwrap();
    
    // For now, we only have one digger: "dig-jon-ai-001"
    let digger = lock.get_digger("dig-jon-ai-001");
    
    let status = match digger {
        Some(d) => RobotStatusResponse {
            available: d.current_contract.is_none(),
            digger_id: d.id.clone(),
            current_contract: d.current_contract.as_ref().map(|c| c.id.clone()),
        },
        None => RobotStatusResponse {
            available: false,
            digger_id: "none".to_string(),
            current_contract: None,
        },
    };

    let json = serde_json::to_string(&status).unwrap();
    let response = Response::from_string(json)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
    
    let _ = request.respond(response);
}

fn handle_stake(mut request: tiny_http::Request) {
    // Read request body
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        let response = Response::from_string(format!("Failed to read body: {}", e))
            .with_status_code(400);
        let _ = request.respond(response);
        return;
    }

    // Parse JSON
    let stake_req: StakeRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            let response = Response::from_string(format!("Invalid JSON: {}", e))
                .with_status_code(400);
            let _ = request.respond(response);
            return;
        }
    };

    println!("💰 Stake received: {} RT for contract {}", stake_req.amount_rt, stake_req.contract_id);

    // Process the stake and calculate contract duration
    let digger_manager = DiggerManager::global();
    
    // Get digger and update its contract
    let mut digger_lock = digger_manager.lock().unwrap();
    let digger = match digger_lock.get_digger_mut("dig-jon-ai-001") {
        Some(d) => d,
        None => {
            let response = Response::from_string("Digger not found")
                .with_status_code(404);
            let _ = request.respond(response);
            return;
        }
    };
    
    let power_kw = digger.power_kw;
    let max_token_throughput = digger.max_token_throughput;
    
    // Calculate contract duration
    // duration_hours = amount_rt / (power_kw × max_token_throughput)
    let duration_hours = stake_req.amount_rt / (power_kw * max_token_throughput as f64);
    
    println!(
        "📊 Calculated duration: {:.2} hours (stake={} RT, power={} kW, throughput={} tokens/sec)",
        duration_hours, stake_req.amount_rt, power_kw, max_token_throughput
    );
    
    // Get or create contract and update with stake info
    let mut contract = digger.current_contract
        .clone()
        .unwrap_or_else(|| {
            // Create new contract if digger doesn't have one
            Contract {
                id: stake_req.contract_id.clone(),
                authorized: true,
                torq: 1, // Default multiplier
                max_token_throughput,
                interval_seconds: 1, // Default: 1 second per milestone
                total_tokens: 0,
                robo_stake_total: 0.0,
                duration_hours: 0.0,
            }
        });
    
    // Update contract with stake and duration
    contract.id = stake_req.contract_id.clone(); // Update contract ID to match stake request
    contract.robo_stake_total = stake_req.amount_rt;
    contract.duration_hours = duration_hours;
    contract.interval_seconds = 5; // 5 second milestones for testing
    
    // Store updated contract in the digger
    digger.current_contract = Some(contract.clone());
    drop(digger_lock); // Release lock before starting execution

    println!("🔍 DEBUG: About to check CONTRACT_STARTER...");
    
    // Start contract execution (if contract starter is set)
    let starter_guard = CONTRACT_STARTER.lock().unwrap();
    println!("🔍 DEBUG: CONTRACT_STARTER lock acquired");
    
    if let Some(starter) = starter_guard.as_ref() {
        println!("🚀 DEBUG: CONTRACT_STARTER is Some! Starting contract execution for {}", contract.id);
        println!("   DEBUG: Calling starter with:");
        println!("     - digger_id: dig-jon-ai-001");
        println!("     - power_kw: {}", power_kw);
        println!("     - max_token_throughput: {}", max_token_throughput);
        println!("     - contract.id: {}", contract.id);
        
        // Drop the lock before calling starter (prevent deadlock)
        drop(starter_guard);
        
        // Call the starter (this spawns an async task in headless_executor)
        println!("🔍 DEBUG: About to call starter function...");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let starter_ref = CONTRACT_STARTER.lock().unwrap();
            if let Some(s) = starter_ref.as_ref() {
                s(
                    "dig-jon-ai-001".to_string(),
                    power_kw,
                    max_token_throughput,
                    contract.clone(),
                );
            }
        })) {
            Ok(_) => {
                println!("✅ DEBUG: Contract starter function returned successfully");
                println!("✅ Contract executor started successfully");
            }
            Err(e) => {
                eprintln!("❌ DEBUG: Contract starter panicked: {:?}", e);
                eprintln!("❌ Contract starter panicked");
                let response = Response::from_string("Internal error starting contract")
                    .with_status_code(500);
                let _ = request.respond(response);
                return;
            }
        }
    } else {
        println!("⚠️  DEBUG: CONTRACT_STARTER is None!");
        println!("⚠️  No contract starter registered - contract will not execute");
    }

    let resp = StakeResponse {
        status: "accepted".to_string(),
        contract_id: stake_req.contract_id.clone(),
        message: format!(
            "Received {} RT for contract, duration: {:.2} hours",
            stake_req.amount_rt, duration_hours
        ),
    };

    let json = serde_json::to_string(&resp).unwrap();
    let response = Response::from_string(json)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
    
    let _ = request.respond(response);
}
