use crate::crypto::DiggerKeypair;
use crate::http_api::ApiState;
use crate::printer_registry::{PrinterCertificate, PrinterRegistration};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};
use tokio_stream::StreamExt;

/// Start listening for printer-related NATS events
pub async fn start_printer_listeners(state: ApiState) {
    // Spawn listener for printer registrations
    let reg_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_printer_registrations(reg_state).await {
            error!("Printer registration listener error: {}", e);
        }
    });

    // Spawn listener for contract_started events
    let start_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_contract_started_events(start_state).await {
            error!("Contract started listener error: {}", e);
        }
    });

    // Spawn listener for contract_completed events
    let complete_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_contract_completed_events(complete_state).await {
            error!("Contract completed listener error: {}", e);
        }
    });

    info!("🎧 Printer event listeners started");
}

/// Handle printer registration requests
async fn handle_printer_registrations(state: ApiState) -> Result<(), Box<dyn std::error::Error>> {
    let mut sub = state
        .nats_client
        .subscribe("printer.register")
        .await?;

    info!("📝 Listening for printer registrations on 'printer.register'");
    
    // Ensure subscription is fully established
    state.nats_client.flush().await?;
    
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

        // Send certificate back to printer
        let cert_json = serde_json::to_vec(&certificate).unwrap();
        if let Some(reply) = msg.reply {
            info!("📤 Sending certificate to reply subject: {}", reply);
            if let Err(e) = state.nats_client.publish(reply, cert_json.clone().into()).await {
                error!("Failed to send certificate: {}", e);
            }
        }

        // Also publish to printer's response topic
        let response_topic = format!("printer.{}.certificate", registration.printer_id);
        info!("📤 Publishing certificate to: {}", response_topic);
        if let Err(e) = state
            .nats_client
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

/// Issue a certificate for a registered printer
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

/// Handle contract_started events from printers
async fn handle_contract_started_events(
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sub = state
        .nats_client
        .subscribe("printer.contract_started")
        .await?;

    info!("🎧 Listening for contract_started events");

    while let Some(msg) = sub.next().await {
        #[derive(Deserialize)]
        struct ContractStarted {
            printer_id: String,
            contract_id: String,
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

/// Handle contract_completed events from printers
async fn handle_contract_completed_events(
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sub = state
        .nats_client
        .subscribe("printer.contract_completed")
        .await?;

    info!("🎧 Listening for contract_completed events");

    while let Some(msg) = sub.next().await {
        #[derive(Deserialize)]
        struct ContractCompleted {
            printer_id: String,
            contract_id: String,
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

        // TODO: Generate JouleTorqOre with capacity value
        // For now just log
        info!(
            "📦 Would generate ore: {} watt-hours for contract {}",
            event.capacity_watt_hours, event.contract_id
        );
    }

    Ok(())
}
