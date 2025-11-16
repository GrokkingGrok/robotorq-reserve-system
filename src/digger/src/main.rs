// Digger v0.2.0 - Pure Backend (No Tauri)
// Phase 1: HTTP Server + SQLite Storage + Hash-Only Transmission

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod contract_state;
mod crypto;
mod http_api;
mod jtu_hasher;
mod jtu_storage;

use config::DiggerConfig;
use contract_state::ContractStateManager;
use crypto::DiggerKeypair;
use jtu_storage::JtuStorageManager;
use http_api::{ApiState, create_router};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "digger=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🤖 Digger v0.2.0 starting...");

    // Load config
    let config = match DiggerConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("Digger ID: {}", config.digger_id);
    tracing::info!("HTTP Port: {}", config.http_port);
    tracing::info!("Storage Path: {}", config.storage_path.display());
    tracing::info!("Batch Interval: {} seconds", config.batch_interval_sec);
    tracing::info!("NATS URL: {}", config.nats_url);

    // Connect to NATS
    tracing::info!("🔌 Connecting to NATS...");
    let nats_client = match async_nats::connect(&config.nats_url).await {
        Ok(client) => {
            tracing::info!("✅ Connected to NATS at {}", config.nats_url);
            client
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to NATS: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize managers
    let contract_manager = ContractStateManager::new();
    let storage_manager = JtuStorageManager::new(config.storage_path.clone())
        .expect("Failed to initialize storage manager");

    // Generate Falcon-1024 keypair for signing ore batches
    tracing::info!("🔐 Generating Falcon-1024 keypair...");
    let keypair = DiggerKeypair::generate();
    tracing::info!("✅ Keypair generated (public key: {} bytes)", keypair.public_key_bytes().len());

    // Create API state
    let state = ApiState::new(config.clone(), contract_manager, storage_manager, nats_client, keypair);

    // Spawn background task for hash transmission
    let hash_sender_state = state.clone();
    let batch_interval = config.batch_interval_sec;
    tokio::spawn(async move {
        hash_sender_task(hash_sender_state, batch_interval).await;
    });

    // Create router with all API endpoints
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    tracing::info!("🚀 Digger HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    tracing::info!("✅ Server ready with full API!");
    
    axum::serve(listener, app)
        .await
        .unwrap();
    
    tracing::info!("Server shutdown");
}

// ============================================================================
// Background Hash Sender Task
// ============================================================================

/// Background task that periodically sends JTU hash batches to NATS
/// 
/// This task runs every `batch_interval_sec` seconds and:
/// 1. Checks which contracts are ready to send hashes (approved + interval elapsed)
/// 2. Retrieves all JTU hashes for each contract from storage
/// 3. Publishes hash batch to NATS subject "ore.batch"
/// 4. Updates contract state with last_hash_send timestamp
async fn hash_sender_task(state: ApiState, batch_interval_sec: u64) {
    tracing::info!("🚀 Hash sender task started (interval: {}s)", batch_interval_sec);
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(batch_interval_sec));
    
    loop {
        interval.tick().await;
        
        // Get contracts ready to send
        let ready_contracts = {
            let manager = state.contract_manager.lock().unwrap();
            manager.contracts_ready_to_send(batch_interval_sec)
        };
        
        if ready_contracts.is_empty() {
            tracing::debug!("No contracts ready to send hashes");
            continue;
        }
        
        tracing::info!("📤 Sending hashes for {} contracts", ready_contracts.len());
        
        for contract_id in ready_contracts {
            // Get all hashes for this contract
            let hashes = {
                let storage = state.storage_manager.lock().unwrap();
                match storage.get_all_hashes(&contract_id) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::error!("Failed to get hashes for {}: {}", contract_id, e);
                        continue;
                    }
                }
            };
            
            if hashes.is_empty() {
                tracing::warn!("No hashes found for contract {}", contract_id);
                continue;
            }
            
            // Build NATS message (hash-only, TOON format in Phase 6)
            let message_data = serde_json::json!({
                "contract_id": contract_id,
                "digger_id": state.config.digger_id,
                "hashes": hashes,
                "hash_count": hashes.len(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            // Sign the batch (Phase 4: Falcon-1024)
            // Hash the entire batch for deterministic signing
            let batch_hash = crate::crypto::hash_ore_for_signing(
                &contract_id,
                &state.config.digger_id,
                0, // milestone index (simplified for now)
                0.0, // joules (not needed for hash batch)
                0.0, // robo_stake (not needed for hash batch)
                &hashes,
                &chrono::Utc::now().to_rfc3339(),
            );
            
            let signature = state.keypair.sign(&batch_hash);
            
            // Complete message with signature
            let signed_message = serde_json::json!({
                "contract_id": contract_id,
                "digger_id": state.config.digger_id,
                "hashes": hashes,
                "hash_count": hashes.len(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "signature": hex::encode(&signature),
                "public_key": hex::encode(state.keypair.public_key_bytes()),
            });
            
            // Publish to NATS
            match state.nats_client
                .publish("ore.batch", signed_message.to_string().into())
                .await 
            {
                Ok(_) => {
                    tracing::info!(
                        "✅ Sent {} hashes for contract {} to NATS (signed with Falcon-1024)",
                        hashes.len(),
                        contract_id
                    );
                    
                    // Mark hash send in contract state
                    let mut manager = state.contract_manager.lock().unwrap();
                    if let Some(contract) = manager.get_mut(&contract_id) {
                        contract.mark_hash_send();
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Failed to send hashes for {}: {}",
                        contract_id,
                        e
                    );
                }
            }
        }
    }
}
