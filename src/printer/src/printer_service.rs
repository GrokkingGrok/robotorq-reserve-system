use anyhow::{Context, Result};
use async_nats::Client;
use chrono::Utc;
use futures_util::StreamExt;
use pqcrypto_traits::sign::PublicKey;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::{Certificate, CertificateManager, Config, KlipperClient};
use crate::metrics::METRICS;

pub struct PrinterService {
    config: Config,
    nats_client: Client,
    klipper_client: KlipperClient,
    cert_manager: CertificateManager,
    is_printing: bool,
    print_start_time: Option<chrono::DateTime<Utc>>,
    current_job_id: Option<String>,
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
        })
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
        } else {
            warn!("⚠️  No certificate found - registering with Digger");
            self.register_with_digger().await?;
        }
        
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("🚀 Starting status reporting loop");
        
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        
        loop {
            interval.tick().await;
            
            // Query printer state
            let currently_printing = self.check_if_printing().await;
            
            // Detect state change
            if currently_printing != self.is_printing {
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
            // Print started
            self.print_start_time = Some(Utc::now());
            self.current_job_id = Some(format!("job-{}", Utc::now().timestamp()));
            
            info!("🖨️  PRINT STARTED");
            info!("   Job ID: {}", self.current_job_id.as_ref().unwrap());
            info!("   Reserving capacity: {}W", self.config.rated_watts);
            
        } else if !now_printing && self.is_printing {
            // Print finished
            if let Some(start_time) = self.print_start_time {
                let duration = Utc::now().signed_duration_since(start_time);
                let duration_hours = duration.num_seconds() as f64 / 3600.0;
                let capacity_kwh = (self.config.rated_watts as f64 / 1000.0) * duration_hours;
                
                info!("✅ PRINT FINISHED");
                info!("   Duration: {:.2} hours", duration_hours);
                info!("   Capacity: {:.4} kWh", capacity_kwh);
                
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
}
