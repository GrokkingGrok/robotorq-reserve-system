// Headless Contract Executor
//
// This module provides contract execution without Tauri/GUI dependencies.
// It replicates the core logic from digger.rs's start_contract() but uses
// tokio instead of tauri::async_runtime and skips GUI event emissions.

use crate::digger::{ContractControl, ContractStateManager, DiggerManager};
use crate::ore_storage::OreStorage;
use crate::refinery_client::send_ore_to_refinery;
use crate::types::{Contract, JouleTorqOre};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

/// Start executing a contract in headless mode (no GUI events)
pub fn start_headless_contract(
    digger_id: String,
    power_kw: f64,
    max_token_throughput: u64,
    contract: Contract,
    ore_store: Arc<Mutex<OreStorage>>,
) {
    tokio::spawn(async move {
        execute_contract_loop(
            digger_id,
            power_kw,
            max_token_throughput,
            contract,
            ore_store,
        )
        .await;
    });
}

async fn execute_contract_loop(
    digger_id: String,
    power_kw: f64,
    max_token_throughput: u64,
    contract: Contract,
    ore_store: Arc<Mutex<OreStorage>>,
) {
    let mut milestone_index: u32 = 0;
    
    // Calculate throughput (tokens per update)
    let throughput = max_token_throughput * contract.torq as u64;
    
    // Calculate contract end time
    let contract_start_time = SystemTime::now();
    let contract_duration = Duration::from_secs_f64(contract.duration_hours * 3600.0);
    let contract_end_time = contract_start_time + contract_duration;
    
    println!(
        "⏱️  Contract {} will run for {:.2} hours (until {:?})",
        contract.id, contract.duration_hours, contract_end_time
    );
    
    // Initialize contract state as Running
    let state_manager = ContractStateManager::global();
    state_manager
        .lock()
        .unwrap()
        .set_state(contract.id.clone(), ContractControl::Running);
    
    let total_milestones =
        ((contract.duration_hours * 3600.0) / contract.interval_seconds as f64) as u32;
    
    println!(
        "🚀 Starting headless contract execution: {} (total milestones: {})",
        contract.id, total_milestones
    );
    
    // Tracking variables
    let mut total_joules: u64 = 0;
    let mut total_tokens: u64 = 0;
    let mut robo_stake_sent: f64 = 0.0;
    
    // Main execution loop
    loop {
        // Check contract state (pause/stop)
        let state = state_manager
            .lock()
            .unwrap()
            .get_state(&contract.id);
        
        match state {
            ContractControl::Stopped | ContractControl::Completed => {
                println!("🛑 Contract {} stopped/completed", contract.id);
                break;
            }
            ContractControl::Paused => {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            ContractControl::Running => {
                // Check if contract time has expired
                if SystemTime::now() >= contract_end_time {
                    println!("✅ Contract {} completed", contract.id);
                    state_manager
                        .lock()
                        .unwrap()
                        .set_state(contract.id.clone(), ContractControl::Completed);
                    break;
                }
            }
        }
        
        // Sleep for the interval
        sleep(Duration::from_secs(contract.interval_seconds as u64)).await;
        
        // Generate ore for this milestone
        let joules = (power_kw * 1000.0 * contract.interval_seconds as f64) as u64;
        let tokens = throughput * contract.interval_seconds as u64;
        
        // Calculate RoboStake for this milestone
        let progress = milestone_index as f64 / total_milestones as f64;
        let milestone_robo_stake = contract.robo_stake_total * (1.0 / total_milestones as f64);
        
        total_joules += joules;
        total_tokens += tokens;
        robo_stake_sent += milestone_robo_stake;
        
        let ore = JouleTorqOre {
            digger_id: digger_id.clone(),
            contract_id: contract.id.clone(),
            tokens_generated: tokens,
            joules,
            milestone_index,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            robo_stake_amount: milestone_robo_stake,
            proof_of_work: None, // Headless mode doesn't generate proofs
            signature: None, // Unsigned in headless mode
        };
        
        println!(
            "⛏️  Milestone {}/{}: {} tokens, {} J, {:.6} RT ({:.1}% complete)",
            milestone_index + 1,
            total_milestones,
            tokens,
            joules,
            milestone_robo_stake,
            progress * 100.0
        );
        
        // Store ore
        ore_store.lock().unwrap().add_ore(ore.clone());
        
        // Send to Refinery
        match send_ore_to_refinery(&ore).await {
            Ok(_) => {
                println!("✅ Milestone {} confirmed - ore sent to Refinery", milestone_index);
            }
            Err(e) => {
                println!("❌ Milestone {} failed to send: {}", milestone_index, e);
            }
        }
        
        milestone_index += 1;
        
        // Check if we've completed all milestones
        if milestone_index >= total_milestones {
            println!("✅ All milestones completed for contract {}", contract.id);
            state_manager
                .lock()
                .unwrap()
                .set_state(contract.id.clone(), ContractControl::Stopped);
            break;
        }
    }
    
    println!(
        "📊 Contract {} summary: {} tokens, {} J, {:.6} RT delivered",
        contract.id, total_tokens, total_joules, robo_stake_sent
    );
    
    // Mark digger as available
    let manager = DiggerManager::global();
    {
        let mut mgr_lock = manager.lock().unwrap();
        if let Some(digger) = mgr_lock.get_digger_mut(&digger_id) {
            digger.current_contract = None;
        }
    }
}
