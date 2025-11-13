use crate::types::{Contract, JouleTorqOre};
use crate::ore_storage::OreStorage;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use std::collections::HashMap;
use base64::{engine::general_purpose, Engine as _};
use tauri::Emitter;


#[derive(Clone)]
pub struct Digger {
    pub id: String,
    pub power_kw: f64,
    pub max_token_throughput: u64,
    pub current_contract: Option<Contract>,
}

impl Digger {
    pub fn start_contract(
        &mut self,
        contract: Contract,
        ore_store: Arc<Mutex<OreStorage>>,
        app: tauri::AppHandle,
        job_id: String,
    ) {
        self.current_contract = Some(contract.clone());
        let digger_id = self.id.clone();
        let power_kw = self.power_kw;
        let throughput =
            self.max_token_throughput * contract.torq as u64; // tokens per interval
        
        tauri::async_runtime::spawn(async move {
            let mut milestone_index: u32 = 0;

            loop {
                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // Simulated photo proof (base64 text)
                let proof_photo = capture_photo_placeholder(&digger_id, milestone_index);

                let tokens = throughput;
                let joules = (power_kw * contract.interval_seconds as f64 * 1000.0) as u64;

                let ore = JouleTorqOre {
                    digger_id: digger_id.clone(),
                    contract_id: contract.id.clone(),
                    tokens_generated: tokens,
                    joules,
                    milestone_index,
                    timestamp: start_time,
                    proof_of_work: proof_photo,
                };

                println!(
                    "⛏️ Digger {} milestone {}: {} tokens, {} joules",
                    digger_id, milestone_index, tokens, joules
                );

                ore_store.lock().unwrap().add_ore(ore.clone());

                // 🔊 Emit live update to frontend (Tauri v2)
                if let Err(e) = app.emit("ore_update", ore.clone()) {
                    println!("⚠️ Failed to emit ore_update: {:?}", e);
                }

                milestone_index += 1;

                

                sleep(Duration::from_secs(contract.interval_seconds)).await;
            }
        });
    }
}

fn capture_photo_placeholder(digger_id: &str, milestone_index: u32) -> Option<String> {
    let data = format!("photo-{}-{}", digger_id, milestone_index);
    Some(general_purpose::STANDARD.encode(data))
}

// ---------------- Digger Manager ----------------

pub struct DiggerManager {
    diggers: HashMap<String, Digger>,
}

impl DiggerManager {
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<DiggerManager>> =
                Arc::new(Mutex::new(DiggerManager { diggers: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    pub fn get_digger_mut(&mut self, id: &str) -> Option<&mut Digger> {
        self.diggers.get_mut(id)
    }

    pub fn list_diggers(&self) -> Vec<String> {
        self.diggers.keys().cloned().collect()
    }

    pub fn add_digger(&mut self, digger: Digger) {
        self.diggers.insert(digger.id.clone(), digger);
    }
}
