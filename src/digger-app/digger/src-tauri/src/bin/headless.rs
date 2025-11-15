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

use digger::DiggerManager;
use types::Contract;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    println!("🤖 Starting Headless Digger...");
    println!("   Mode: HTTP API only (no GUI)");
    println!("   Port: 9000");
    println!("   Endpoints:");
    println!("     GET  /robot/status");
    println!("     POST /stake");
    println!("     GET  /health");
    
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
    
    // Start HTTP server in background task
    tokio::spawn(async {
        println!("\n🚀 Starting HTTP server...");
        http_api::start_http_server();
    });
    
    println!("✅ Headless Digger ready for requests\n");
    
    // Keep main thread alive
    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    println!("\n🛑 Shutting down Headless Digger...");
}
