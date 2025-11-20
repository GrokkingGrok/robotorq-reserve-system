use anyhow::{Context, Result};
use async_nats::Client;
use chrono::Utc;
use futures_util::StreamExt;
use pqcrypto_traits::sign::PublicKey;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{Certificate, CertificateManager, Config, KlipperClient, MockKlipperClient};
use crate::metrics::METRICS;
use crate::mock_api::MockContractState;

pub struct PrinterService {
    config: Config,
    nats_client: Client,
    klipper_client: KlipperClient,
    cert_manager: CertificateManager,
    is_printing: bool,
    print_start_time: Option<chrono::DateTime<Utc>>,
    current_job_id: Option<String>,
    assigned_contract_id: Option<String>,  // Contract assigned by Digger
    mock_contract_state: Option<Arc<RwLock<MockContractState>>>,  // Shared with mock API
    mock_klipper_client: Option<Arc<MockKlipperClient>>,  // Shared with mock API
}

impl PrinterService {
    pub async fn new(config: Config) -> Result<Self> {
        // Connect to NATS
        let nats_client = async_nats::connect(&config.nats_url)
            .await
            .context("Failed to connect to NATS")?;
        
        info!("✅ Connected to NATS at {}", config.nats_url);
        
        // Initialize Klipper client
        let klipper_client = KlipperClient::new(&config.klipper_url);
        
        // Initialize certificate manager
        let cert_manager = CertificateManager::new(&config.printer_id);
        
        Ok(Self {
            config,
            nats_client,
            klipper_client,
            cert_manager,
            is_printing: false,
            print_start_time: None,
            current_job_id: None,
            assigned_contract_id: None,
            mock_contract_state: None,
            mock_klipper_client: None,
        })
    }
    
    pub fn set_mock_contract_state(&mut self, state: Arc<RwLock<MockContractState>>) {
        self.mock_contract_state = Some(state);
    }
    
    pub fn set_mock_klipper_client(&mut self, client: Arc<MockKlipperClient>) {
        info!("🔧 Setting shared MockKlipperClient");
        self.mock_klipper_client = Some(client);
    }
    
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    pub async fn initialize(&mut self) -> Result<()> {
        // Try to load existing certificate
        if self.cert_manager.load_certificate().is_ok() {
            info!("✅ Loaded existing bonded certificate");
            if let Some(cert) = self.cert_manager.certificate() {
                info!("   Certificate Hash: {}", cert.certificate_hash);
                info!("   Issued At: {}", cert.issued_at);
                info!("   Certified Capacity: {}W", cert.rated_watts);
                METRICS.certificate_valid.set(1.0);
            }
            
            // TODO: Notify Digger we're online (re-register without requesting new cert)
            info!("📡 Using existing certificate - skipping re-registration for now");
        } else {
            warn!("⚠️  No certificate found - requesting from Digger");
            self.register_with_digger().await?;
        }
        
        // Note: Contract listener will be started after mock state is set (in main.rs)
        
        Ok(())
    }
    
    /// Start the contract listener (call this after setting mock_contract_state)
    pub async fn start_listener(&self) -> Result<()> {
        self.start_contract_listener().await
    }
    
    /// Manually trigger registration with Digger (call this explicitly, not automatic)
    pub async fn register(&mut self) -> Result<()> {
        self.register_with_digger().await
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("🚀 Starting status reporting loop");
        
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        
        loop {
            interval.tick().await;
            
            debug!("⏰ Heartbeat tick - checking printer state");
            
            // Query printer state
            let currently_printing = self.check_if_printing().await;
            
            // Detect state change
            if currently_printing != self.is_printing {
                info!("🔄 State change detected: was_printing={}, now_printing={}", 
                      self.is_printing, currently_printing);
                if let Err(e) = self.handle_state_change(currently_printing).await {
                    error!("Error handling state change: {}", e);
                }
            }
            
            self.is_printing = currently_printing;
            
            // Send heartbeat
            if let Err(e) = self.send_status().await {
                error!("Error sending status: {}", e);
            }
            
            // Update uptime metric
            METRICS.uptime_seconds.inc();
        }
    }
    
    async fn check_if_printing(&self) -> bool {
        // If mock mode and shared client is available, use it
        if self.config.mock_mode {
            if let Some(ref mock_client) = self.mock_klipper_client {
                let is_printing = mock_client.is_printing().await.unwrap_or(false);
                debug!("🔍 Mock mode: check_if_printing = {}", is_printing);
                return is_printing;
            } else {
                warn!("⚠️  Mock mode enabled but mock_klipper_client is None!");
            }
        }
        
        // Otherwise use regular Klipper client
        match self.klipper_client.is_printing().await {
            Ok(printing) => printing,
            Err(e) => {
                debug!("Could not query Klipper: {}", e);
                false
            }
        }
    }
    
    async fn handle_state_change(&mut self, now_printing: bool) -> Result<()> {
        if now_printing && !self.is_printing {
            // Print started - check mock contract state first
            let contract_id = if let Some(ref mock_state) = self.mock_contract_state {
                mock_state.read().await.assigned_contract_id.clone()
            } else {
                self.assigned_contract_id.clone()
            };
            
            // Store contract_id in service for this print session
            if let Some(ref cid) = contract_id {
                self.assigned_contract_id = Some(cid.clone());
            }
            
            self.print_start_time = Some(Utc::now());
            self.current_job_id = Some(format!("job-{}", Utc::now().timestamp()));
            
            info!("🖨️  PRINT STARTED");
            info!("   Job ID: {}", self.current_job_id.as_ref().unwrap());
            info!("   Contract: {}", contract_id.as_deref().unwrap_or("none"));
            info!("   Reserving capacity: {}W", self.config.rated_watts);
            
            // Publish contract_started event if we have an assigned contract
            if let Some(ref cid) = contract_id {
                self.publish_contract_started(cid).await?;
            }
            
        } else if !now_printing && self.is_printing {
            // Print finished
            if let Some(start_time) = self.print_start_time {
                let duration = Utc::now().signed_duration_since(start_time);
                let duration_secs = duration.num_seconds() as f64;
                let duration_hours = duration_secs / 3600.0;
                let capacity_watt_hours = self.config.rated_watts as f64 * duration_hours;
                let capacity_kwh = capacity_watt_hours / 1000.0;
                
                info!("✅ PRINT FINISHED");
                info!("   Duration: {:.2} hours ({:.1}s)", duration_hours, duration_secs);
                info!("   Capacity: {:.2} Wh ({:.4} kWh)", capacity_watt_hours, capacity_kwh);
                
                // Publish contract_completed event if we have an assigned contract
                if let Some(ref contract_id) = self.assigned_contract_id {
                    self.publish_contract_completed(contract_id, duration_secs, capacity_watt_hours).await?;
                    info!("   Contract {} completed", contract_id);
                }
                
                // Clear assignment after completion (both service and mock state)
                if let Some(ref mock_state) = self.mock_contract_state {
                    mock_state.write().await.assigned_contract_id = None;
                }
                self.assigned_contract_id = None;
                
                self.report_completion(duration_hours, capacity_kwh).await?;
                
                // Update metrics
                METRICS.prints_completed_total.inc();
                METRICS.capacity_kwh_total.inc_by(capacity_kwh);
            }
            
            self.print_start_time = None;
            self.current_job_id = None;
        }
        
        Ok(())
    }
    
    async fn send_status(&self) -> Result<()> {
        let status = json!({
            "printer_id": self.config.printer_id,
            "is_printing": self.is_printing,
            "rated_watts": self.config.rated_watts,
            "current_job_id": self.current_job_id,
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        self.nats_client
            .publish("printer.status", status.to_string().into())
            .await?;
        
        METRICS.heartbeats_sent_total.inc();
        
        let emoji = if self.is_printing { "🖨️ " } else { "💤" };
        let state = if self.is_printing { "PRINTING" } else { "IDLE" };
        debug!("{} Status: {}", emoji, state);
        
        Ok(())
    }
    
    async fn register_with_digger(&mut self) -> Result<()> {
        // Generate Falcon-1024 keypair for this printer
        let (public_key, _secret_key) = pqcrypto_falcon::falcon1024::keypair();
        let public_key_hex = hex::encode(public_key.as_bytes());
        
        let registration = json!({
            "printer_id": self.config.printer_id,
            "model": self.config.printer_model,
            "manufacturer": "RoboTorq",
            "serial_number": format!("{}-001", self.config.printer_id), // Mock serial
            "rated_watts": self.config.rated_watts,
            "public_key": public_key_hex,
        });
        
        info!("📤 Requesting bonded certificate from Digger");
        
        // Properly encode as JSON bytes
        let registration_bytes = serde_json::to_vec(&registration)?;
        
        self.nats_client
            .publish("printer.register", registration_bytes.into())
            .await?;
        
        // Subscribe to certificate response
        let cert_topic = format!("printer.{}.certificate", self.config.printer_id);
        let mut sub = self.nats_client.subscribe(cert_topic.clone()).await?;
        
        info!("📥 Waiting for certificate on topic: {}", cert_topic);
        
        // Wait for certificate (30 second timeout)
        match tokio::time::timeout(Duration::from_secs(30), sub.next()).await {
            Ok(Some(msg)) => {
                let cert: Certificate = serde_json::from_slice(&msg.payload)?;
                self.cert_manager.save_certificate(&cert)?;
                
                info!("✅ Received and saved bonded certificate");
                info!("   Certificate Hash: {}", cert.certificate_hash);
                info!("   Certified Capacity: {}W", cert.rated_watts);
                
                METRICS.certificate_valid.set(1.0);
                
                Ok(())
            }
            Ok(None) => {
                error!("❌ Certificate stream closed unexpectedly");
                anyhow::bail!("Certificate stream closed")
            }
            Err(_) => {
                error!("❌ Timeout waiting for certificate (30s)");
                error!("   Check that Digger service is running");
                anyhow::bail!("Timeout waiting for certificate")
            }
        }
    }
    
    async fn report_completion(&self, duration_hours: f64, capacity_kwh: f64) -> Result<()> {
        let completion = json!({
            "printer_id": self.config.printer_id,
            "job_id": self.current_job_id,
            "duration_hours": duration_hours,
            "capacity_kwh": capacity_kwh,
            "rated_watts": self.config.rated_watts,
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        self.nats_client
            .publish("printer.completion", completion.to_string().into())
            .await?;
        
        info!("📤 Sent completion report to Digger");
        
        Ok(())
    }
    
    /// Start listening for contract assignments from Digger
    async fn start_contract_listener(&self) -> Result<()> {
        let printer_id = self.config.printer_id.clone();
        let nats_client = self.nats_client.clone();
        let mock_contract_state = self.mock_contract_state.clone();
        
        // Spawn background task to listen for assignments
        tokio::task::spawn_local(async move {
            let assignment_topic = format!("digger.contract.assigned.{}", printer_id);
            
            match nats_client.subscribe(assignment_topic.clone()).await {
                Ok(mut sub) => {
                    info!("📡 Listening for contract assignments on: {}", assignment_topic);
                    
                    while let Some(msg) = sub.next().await {
                        match serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                            Ok(assignment) => {
                                if let Some(contract_id) = assignment.get("contract_id").and_then(|v| v.as_str()) {
                                    info!("📋 Contract assigned by Digger: {}", contract_id);
                                    
                                    // Update mock contract state so handle_state_change can access it
                                    if let Some(ref state) = mock_contract_state {
                                        let mut s = state.write().await;
                                        s.assigned_contract_id = Some(contract_id.to_string());
                                        info!("✅ Updated mock contract state with: {}", contract_id);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse contract assignment: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe to contract assignments: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Publish contract_started event to Digger
    async fn publish_contract_started(&self, contract_id: &str) -> Result<()> {
        let event = json!({
            "printer_id": self.config.printer_id,
            "contract_id": contract_id,
            "started_at": Utc::now().to_rfc3339(),
        });
        
        self.nats_client
            .publish("printer.contract_started", serde_json::to_vec(&event)?.into())
            .await?;
        
        info!("📤 Published contract_started event for contract {}", contract_id);
        
        Ok(())
    }
    
    /// Publish contract_completed event to Digger with capacity
    async fn publish_contract_completed(&self, contract_id: &str, duration_secs: f64, capacity_watt_hours: f64) -> Result<()> {
        let event = json!({
            "printer_id": self.config.printer_id,
            "contract_id": contract_id,
            "completed_at": Utc::now().to_rfc3339(),
            "duration_secs": duration_secs,
            "capacity_watt_hours": capacity_watt_hours,
        });
        
        self.nats_client
            .publish("printer.contract_completed", serde_json::to_vec(&event)?.into())
            .await?;
        
        info!("📤 Published contract_completed event for contract {} - {:.2} Wh", contract_id, capacity_watt_hours);
        
        Ok(())
    }
}
