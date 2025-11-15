// Headless Digger - Standalone HTTP server for E2E testing
//
// This binary runs the Digger HTTP API without Tauri/GUI overhead,
// enabling true headless E2E testing in CI/CD pipelines.
//
// Usage:
//   cargo build --bin headless
//   ./target/debug/headless
//
// Endpoints:
//   GET  /robot/status → Check if robot is available
//   POST /stake        → Receive RoboStake for a contract  
//   GET  /health       → Health check

// Include all Digger modules
#[path = "../digger.rs"]
mod digger;

#[path = "../ore_storage.rs"]
mod ore_storage;

#[path = "../types.rs"]
mod types;

#[path = "../http_api.rs"]
mod http_api;

#[path = "../crypto.rs"]
mod crypto;

#[path = "../refinery_client.rs"]
mod refinery_client;

#[path = "../headless_executor.rs"]
mod headless_executor;

use digger::DiggerManager;
use types::Contract;
use ore_storage::OreStorage;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    println!("🤖 Starting Headless Digger...");
    println!("   Mode: HTTP API only (no GUI)");
    println!("   Port: 9000");
    
    // Get the current tokio runtime handle
    let runtime_handle = tokio::runtime::Handle::current();
    
    // Initialize test digger (same as main.rs)
    {
        let diggers = DiggerManager::global();
        let mut d = diggers.lock().unwrap();
        
        let initial_contract = Contract {
            id: "contract-001".to_string(),
            authorized: true,
            torq: 5,
            max_token_throughput: 12,
            interval_seconds: 5,
            total_tokens: 0,
            robo_stake_total: 0.0,
            duration_hours: 0.0,
        };
        
        d.add_digger(digger::Digger {
            id: "dig-jon-ai-001".to_string(),
            power_kw: 0.25,
            max_token_throughput: 12,
            current_contract: Some(initial_contract),
        });
        
        println!("✅ Initialized digger: dig-jon-ai-001");
        println!("   Power: 0.25 kW");
        println!("   Max Throughput: 12 tokens/sec");
    }
    
    // Create OreStorage for headless contract execution
    let ore_store = OreStorage::global();
    let ore_store_for_callback = Arc::clone(&ore_store);
    let handle_for_callback = runtime_handle.clone();

    println!("🔍 DEBUG: About to register contract starter callback...");
    
    // Register contract starter callback (dependency injection)
    http_api::set_contract_starter(move |digger_id, power_kw, max_token_throughput, contract| {
        println!("🎯 DEBUG: ===== CONTRACT STARTER CALLBACK INVOKED! =====");
        println!("   Digger: {}", digger_id);
        println!("   Contract: {}", contract.id);
        println!("   Power: {} kW", power_kw);
        println!("   Throughput: {} tokens/sec", max_token_throughput);
        println!("   Calling headless_executor::start_headless_contract()...");
        
        headless_executor::start_headless_contract(
            handle_for_callback.clone(),
            digger_id,
            power_kw,
            max_token_throughput,
            contract,
            ore_store_for_callback.clone(),
        );
        
        println!("   start_headless_contract() returned");
        println!("🎯 DEBUG: ===== CONTRACT STARTER CALLBACK COMPLETE =====");
    });

    println!("✅ DEBUG: Contract starter registered successfully");
    println!("✅ Contract starter registered");
    
    // Start HTTP server (it spawns its own thread)
    println!("\n🚀 Starting HTTP server...");
    http_api::start_http_server();
    
    println!("✅ Headless Digger ready for requests");
    println!("📋 Endpoints:");
    println!("   GET  /robot/status    - Digger status");
    println!("   GET  /health          - Health check");
    println!("   POST /stake           - Accept RoboTorq stake for contract");
    println!("\n🛑 Press Ctrl+C to shutdown gracefully\n");
    
    // Keep main thread alive - simple loop instead of signal (which requires feature flag)
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
