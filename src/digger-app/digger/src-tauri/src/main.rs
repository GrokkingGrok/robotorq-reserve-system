#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod digger;
mod ore_storage;
mod contracts;
mod types;

use digger::DiggerManager;
use contracts::ContractManager;
use ore_storage::OreStorage;

/// Start a contract job
#[tauri::command]
fn start_contract(digger_id: String, contract_id: String, app: tauri::AppHandle) -> Result<String, String> {
    println!("🔹 start_contract called with digger_id={} contract_id={}", digger_id, contract_id);

    let manager = DiggerManager::global();
    let contract_mgr = ContractManager::global();
    let ore_store = OreStorage::global();

    let mut manager_lock = manager.lock().unwrap();
    let manager_ref = match manager_lock.get_digger_mut(&digger_id) {
        Some(d) => d,
        None => {
            println!("❌ Digger {} not found", digger_id);
            return Err(format!("Digger {} not found", digger_id));
        }
    };

    let contract = {
        let contract_mgr_lock = contract_mgr.lock().unwrap();
        match contract_mgr_lock.get_contract(&contract_id) {
            Some(c) => c,
            None => {
                println!("❌ Contract {} not found", contract_id);
                return Err(format!("Contract {} not found", contract_id));
            }
        }
    };

    if !contract.authorized {
        println!("⚠️ Contract {} is not authorized yet", contract_id);
        return Err("Contract not authorized yet".into());
    }

    let job_id = format!("{}-{}", digger_id, contract_id); // create a unique job id
    println!("✅ Starting contract {} on digger {} as job_id {}", contract_id, digger_id, job_id);

    // Pass AppHandle and job_id into start_contract
    manager_ref.start_contract(contract, ore_store, app.clone(), job_id);

    Ok(format!("Digger_{}_started_contract_{}", digger_id, contract_id))
}


#[tauri::command]
fn list_diggers() -> Vec<String> {
    let mgr = DiggerManager::global();
    let lock = mgr.lock().unwrap();
    lock.list_diggers()
}

#[tauri::command]
fn list_contracts() -> Vec<String> {
    let mgr = ContractManager::global();
    let lock = mgr.lock().unwrap();
    lock.list_contracts()
}

fn main() {
    env_logger::init();

    // ---- Mock data bootstrapping ----
    {
        let diggers = DiggerManager::global();
        let mut d = diggers.lock().unwrap();
        d.add_digger(digger::Digger {
            id: "dig-jon-ai-001".to_string(),
            power_kw: 0.25,
            max_token_throughput: 12,
            current_contract: None,
        });
    }

    {
        let contracts = ContractManager::global();
        let mut c = contracts.lock().unwrap();
        c.add_contract(types::Contract {
            id: "contract-001".to_string(),
            authorized: true,
            torq: 5,
            max_token_throughput: 12,
            interval_seconds: 5,
            total_tokens: 0,
        });
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            start_contract,
            list_diggers,
            list_contracts
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
