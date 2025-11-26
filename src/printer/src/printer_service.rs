use anyhow::{Context, Result};
use async_nats::Client;
use chrono::Utc;
use futures_util::StreamExt;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{Config, KlipperClient, MockKlipperClient};
use crate::metrics::METRICS;
use crate::mock_api::MockContractState;

pub struct PrinterService {
    config: Config,
    nats_client: Client,
    klipper_client: KlipperClient,
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
        
        Ok(Self {
            config,
            nats_client,
            klipper_client,
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
        // Simplified initialization for MVP (no certificate / signing)
        info!("🔑 Skipping certificate registration (MVP without signing)");
        Ok(())
    }
    
    /// Start the contract listener (call this after setting mock_contract_state)
    pub async fn start_listener(&self) -> Result<()> {
        self.start_contract_listener().await
    }
    
    // Registration removed for MVP
    
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

            // Update printing state gauge (0 = idle, 1 = printing)
            METRICS.currently_printing.set(if self.is_printing { 1 } else { 0 });
            
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
            let contract_id: Option<String> = if let Some(ref mock_state) = self.mock_contract_state {
                mock_state.read().await.assigned_contract_id.clone()
            } else {
                self.assigned_contract_id.clone()
            };
            
            // Store contract_id in service for this print session
            if let Some(cid) = contract_id.clone() {
                self.assigned_contract_id = Some(cid);
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
                    let mut s: tokio::sync::RwLockWriteGuard<MockContractState> = mock_state.write().await;
                    s.assigned_contract_id = None;
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
    
    // Registration & certificate logic removed for MVP
    
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
