//! Asynchronous NATS event handlers for printer lifecycle integration.
//!
//! Dedicated connections per handler isolate backpressure and avoid head-of-line
//! blocking across different subjects (`printer.register`, `printer.contract_started`,
//! `printer.contract_completed`). Each handler performs minimal parsing,
//! validation, state mutation, and publishing follow-up messages (e.g., certificates).
//!
//! Design considerations:
//! * Registration generates & signs a printer certificate (currently placeholder
//!   signature workflow; future Falcon signing for certificate fields).
//! * Contract completion triggers internal execution to produce JTUs & ore.
//! * Long-running loops include simple timeout instrumentation to surface stalls.
//! * Internal helper `execute_contract_internal` mirrors HTTP execute logic for
//!   reuse without duplicate code paths.
//!
//! Future enhancements:
//! * Structured error metrics and retry backoff on transient failures.
//! * Printer capability negotiation (rated watts vs actual load).
//! * Event schema versioning and signature verification on inbound messages.
use crate::crypto::DiggerKeypair;
use crate::http_api::ApiState;
use crate::printer_registry::{PrinterCertificate, PrinterRegistration};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};
use tokio_stream::StreamExt;

/// Spawn asynchronous tasks for all printer-related NATS subjects.
pub async fn start_printer_listeners(state: ApiState, nats_url: String) {
    // Optional disable of initial printer handshake via env flag.
    // Set DISABLE_PRINTER_HANDSHAKE=1 (or "true") to skip the registration listener
    // and rely solely on a pre-populated `printer_registry.json` plus start/stop
    // contract events. This supports the simplified MVP without live registration.
    let disable_handshake = std::env::var("DISABLE_PRINTER_HANDSHAKE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if disable_handshake {
        info!("🛑 Printer registration handshake disabled (DISABLE_PRINTER_HANDSHAKE set). Expect pre-populated registry file.");
    } else {
        // Spawn listener for printer registrations with its own NATS connection
        let reg_state = state.clone();
        let reg_nats_url = nats_url.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_printer_registrations(reg_state, reg_nats_url).await {
                error!("Printer registration listener error: {}", e);
            }
        });
    }

    // Spawn listener for contract_started events with its own NATS connection
    let start_state = state.clone();
    let start_nats_url = nats_url.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_contract_started_events(start_state, start_nats_url).await {
            error!("Contract started listener error: {}", e);
        }
    });

    // Spawn listener for contract_completed events with its own NATS connection
    let complete_state = state.clone();
    let complete_nats_url = nats_url.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_contract_completed_events(complete_state, complete_nats_url).await {
            error!("Contract completed listener error: {}", e);
        }
    });

    info!("🎧 Printer event listeners started");
}

/// Listen for `printer.register` events, issue certificates, and persist printer state.
async fn handle_printer_registrations(state: ApiState, nats_url: String) -> Result<(), Box<dyn std::error::Error>> {
    // Create dedicated NATS connection for this handler
    info!("🔌 Connecting to NATS for printer registration handler...");
    let nats_client = async_nats::connect(&nats_url).await?;
    info!("✅ Registration handler connected to NATS");
    
    let mut sub = nats_client
        .subscribe("printer.register")
        .await?;

    info!("📝 Listening for printer registrations on 'printer.register'");
    
    // Ensure subscription is fully established
    nats_client.flush().await?;
    
    info!("✅ Subscription created, entering handler loop");
    info!("🔍 DEBUG: About to enter while loop");

    loop {
        info!("🔍 DEBUG: Calling sub.next().await...");
        
        // Try with a timeout to see if .next() is stuck
        let timeout_duration = tokio::time::Duration::from_secs(5);
        let result = tokio::time::timeout(timeout_duration, sub.next()).await;
        
        let msg = match result {
            Ok(Some(m)) => {
                info!("📨 RECEIVED MESSAGE on printer.register");
                info!("   Payload size: {} bytes", m.payload.len());
                m
            }
            Ok(None) => {
                warn!("❌ Subscription closed unexpectedly");
                break;
            }
            Err(_) => {
                info!("⏱️  Timeout waiting for message (5s) - still listening...");
                continue;
            }
        };
                
        let registration: PrinterRegistration = match serde_json::from_slice(&msg.payload) {
            Ok(reg) => reg,
            Err(e) => {
                error!("Failed to parse registration: {}", e);
                continue;
            }
        };

        info!("📥 Printer registration received: {}", registration.printer_id);

        // Validate model
        if !crate::printer_registry::PrinterRegistry::validate_model(&registration.model) {
            warn!("⚠️  Unknown printer model: {}", registration.model);
            // Continue anyway for demo
        }

        // Generate certificate
        let certificate = match issue_certificate(&registration, &state.keypair).await {
            Ok(cert) => cert,
            Err(e) => {
                error!("Failed to issue certificate: {}", e);
                continue;
            }
        };

        // Register printer
        if let Err(e) = state
            .printer_registry
            .register(registration.clone(), certificate.clone())
            .await
        {
            error!("Failed to register printer: {}", e);
            continue;
        }

        info!("✅ Printer registered: {}", registration.printer_id);

        // Send certificate back to printer using dedicated connection
        let cert_json = serde_json::to_vec(&certificate).unwrap();
        if let Some(reply) = msg.reply {
            info!("📤 Sending certificate to reply subject: {}", reply);
            if let Err(e) = nats_client.publish(reply, cert_json.clone().into()).await {
                error!("Failed to send certificate: {}", e);
            }
        }

        // Also publish to printer's response topic
        let response_topic = format!("printer.{}.certificate", registration.printer_id);
        info!("📤 Publishing certificate to: {}", response_topic);
        if let Err(e) = nats_client
            .publish(response_topic, cert_json.into())
            .await
        {
            error!("Failed to publish certificate: {}", e);
        } else {
            info!("✅ Certificate sent successfully");
        }
    }

    Ok(())
}

/// Build and sign a printer certificate from registration data.
async fn issue_certificate(
    registration: &PrinterRegistration,
    keypair: &DiggerKeypair,
) -> Result<PrinterCertificate, String> {
    let issued_at = chrono::Utc::now().to_rfc3339();

    // Build certificate
    let cert = PrinterCertificate {
        printer_id: registration.printer_id.clone(),
        model: registration.model.clone(),
        rated_watts: registration.rated_watts,
        issued_at: issued_at.clone(),
        expires_at: None, // No expiration for demo
        public_key: registration.public_key.clone(),
        certificate_hash: String::new(), // Placeholder
        signature: String::new(),        // Placeholder
    };

    // Calculate certificate hash
    let cert_data = format!(
        "{}|{}|{}|{}|{}",
        cert.printer_id, cert.model, cert.rated_watts, cert.issued_at, cert.public_key
    );
    let mut hasher = Sha256::new();
    hasher.update(cert_data.as_bytes());
    let hash = hasher.finalize();
    let certificate_hash = hex::encode(hash);

    // Sign certificate hash
    let signature = keypair.sign(certificate_hash.as_bytes());

    // Build final certificate
    let final_cert = PrinterCertificate {
        certificate_hash,
        signature: hex::encode(signature),
        ..cert
    };

    Ok(final_cert)
}

/// Subscribe to `printer.contract_started` events and log commencement.
async fn handle_contract_started_events(
    _state: ApiState,
    nats_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create dedicated NATS connection for this handler
    let nats_client = async_nats::connect(&nats_url).await?;
    
    let mut sub = nats_client
        .subscribe("printer.contract_started")
        .await?;

    info!("🎧 Listening for contract_started events");

    while let Some(msg) = sub.next().await {
        #[derive(Deserialize)]
        struct ContractStarted {
            printer_id: String,
            contract_id: String,
            #[allow(dead_code)]
            started_at: String,
        }

        let event: ContractStarted = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to parse contract_started: {}", e);
                continue;
            }
        };

        info!(
            "🖨️  Contract started: {} on printer {}",
            event.contract_id, event.printer_id
        );

        // TODO: Update contract state to "printing"
    }

    Ok(())
}

/// Subscribe to `printer.contract_completed` events, update printer stats, and execute contract work.
async fn handle_contract_completed_events(
    state: ApiState,
    nats_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create dedicated NATS connection for this handler
    let nats_client = async_nats::connect(&nats_url).await?;
    
    let mut sub = nats_client
        .subscribe("printer.contract_completed")
        .await?;

    info!("🎧 Listening for contract_completed events");

    while let Some(msg) = sub.next().await {
        #[derive(Deserialize)]
        struct ContractCompleted {
            printer_id: String,
            contract_id: String,
            #[allow(dead_code)]
            completed_at: String,
            duration_secs: f64,
            capacity_watt_hours: f64,
        }

        let event: ContractCompleted = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to parse contract_completed: {}", e);
                continue;
            }
        };

        info!(
            "✅ Contract completed: {} on printer {} - {:.2} watt-hours",
            event.contract_id, event.printer_id, event.capacity_watt_hours
        );

        // Update printer stats
        if let Err(e) = state
            .printer_registry
            .complete_print(&event.printer_id, event.capacity_watt_hours)
            .await
        {
            error!("Failed to update printer stats: {}", e);
        }

        // Execute contract to generate JTUs and ore
        info!(
            "🏭 Executing contract {} - duration: {:.1}s",
            event.contract_id, event.duration_secs
        );
        
        // Call the execute_contract logic
        use crate::http_api::ExecuteContractRequest;
        let execute_req = ExecuteContractRequest {
            contract_id: event.contract_id.clone(),
            duration_seconds: event.duration_secs.ceil() as u64,
        };
        
        // We need to call the execute logic directly
        match execute_contract_internal(&state, execute_req).await {
            Ok(response) => {
                info!(
                    "✨ Contract {} executed: {} JTUs generated, {:.2} RT ore ({:.1}% of target)",
                    event.contract_id,
                    response.jtus_generated,
                    response.ore_generated,
                    (response.ore_generated / response.ore_target) * 100.0
                );
                
                if response.target_reached {
                    info!("🎯 Ore target reached for contract {}!", event.contract_id);
                }
            }
            Err(e) => {
                error!("Failed to execute contract {}: {}", event.contract_id, e);
            }
        }
    }

    Ok(())
}

/// Internal contract execution shared by NATS completion handler & HTTP endpoint.
async fn execute_contract_internal(
    state: &crate::http_api::ApiState,
    req: crate::http_api::ExecuteContractRequest,
) -> Result<crate::http_api::ExecuteContractResponse, String> {
    use crate::contract_state::ApprovalStatus;
    
    info!(
        "Executing contract: {} for {} seconds",
        req.contract_id, req.duration_seconds
    );

    let mut manager = state.contract_manager.lock().unwrap();
    
    let contract = match manager.get_mut(&req.contract_id) {
        Some(c) => c,
        None => {
            return Err(format!("Contract {} not found", req.contract_id));
        }
    };

    // Must be approved
    if contract.approval_status != ApprovalStatus::StakeApproved
        && contract.approval_status != ApprovalStatus::ExecutionComplete
    {
        return Err(format!(
            "Contract {} not approved (status: {:?})",
            req.contract_id, contract.approval_status
        ));
    }

    // Generate JTUs
    let tokens_per_sec = 1.0;
    let jtus_per_sec = (tokens_per_sec * contract.power_watts) as i64;
    let jtus_generated = jtus_per_sec * req.duration_seconds as i64;
    
    let ore_value_per_jtu = contract.robo_stake / contract.ore_target;
    let ore_generated = jtus_generated as f64 * ore_value_per_jtu;
    
    let total_joules = contract.power_watts * req.duration_seconds as f64;
    let joules_per_jtu = total_joules / jtus_generated as f64;
    let robo_stake_per_jtu = ore_value_per_jtu;
    
    let starting_index = contract.jtu_count;
    let now = chrono::Utc::now().timestamp();
    let contract_id = req.contract_id.clone();
    let digger_id = state.config.digger_id.clone();
    
    // Generate JTUs in parallel
    use rayon::prelude::*;
    let jtus: Vec<crate::jtu_storage::JouleTorqUnit> = (0..jtus_generated)
        .into_par_iter()
        .map(|i| {
            let token_index = starting_index + i;
            let token_id = format!("{}-t{}", contract_id, token_index);
            
            crate::jtu_storage::JouleTorqUnit {
                hash: crate::jtu_hasher::calculate_jtu_hash(
                    &token_id,
                    joules_per_jtu,
                    robo_stake_per_jtu,
                    now,
                    &contract_id,
                    &digger_id,
                ),
                signature: crate::jtu_hasher::create_placeholder_signature(),
                digger_id: digger_id.clone(),
                contract_id: contract_id.clone(),
                token_index,
                milestone_index: 0,
                timestamp: now,
                joules_consumed: joules_per_jtu,
                robo_stake_paid: robo_stake_per_jtu,
            }
        })
        .collect();
    
    // Store JTUs
    {
        let storage = state.storage_manager.lock().unwrap();
        if let Err(e) = storage.insert_batch(&jtus) {
            return Err(format!("Failed to store JTUs: {}", e));
        }
    }
    
    // Record metrics
    state.metrics.contracts_executed_total.inc();
    state.metrics.jtus_generated_total.inc_by(jtus_generated as u64);
    state.metrics.jtus_stored_total.inc_by(jtus_generated as u64);
    state.metrics.ore_generated_total.inc_by(ore_generated);
    
    info!("Stored {} JTUs in database for contract {}", jtus_generated, req.contract_id);

    // Update contract state
    contract.add_jtus(jtus_generated, ore_generated);

    let ore_target = contract.ore_target;
    let total_ore = contract.ore_generated;
    let target_reached = contract.is_ore_target_reached();

    Ok(crate::http_api::ExecuteContractResponse {
        contract_id: req.contract_id,
        jtus_generated,
        ore_generated: total_ore,
        ore_target,
        target_reached,
        message: if target_reached {
            "Ore target reached! Contract ready for minting.".to_string()
        } else {
            format!("{:.1}% of ore target reached", (total_ore / ore_target) * 100.0)
        },
    })
}
