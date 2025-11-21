use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use tracing::info;

/// Printer registration request from printer service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterRegistration {
    pub printer_id: String,
    pub model: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub rated_watts: u32,
    pub public_key: String, // Falcon-1024 public key (hex)
}

/// Certificate issued to printer after registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterCertificate {
    pub printer_id: String,
    pub model: String,
    pub rated_watts: u32,
    pub issued_at: String,
    pub expires_at: Option<String>, // None = no expiration
    pub public_key: String,
    pub certificate_hash: String,   // SHA256 of certificate fields
    pub signature: String,          // Digger signature (hex)
}

/// Bonded printer - registered and certified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondedPrinter {
    pub registration: PrinterRegistration,
    pub certificate: PrinterCertificate,
    pub assigned_contract: Option<String>, // Current contract ID
    pub prints_completed: u64,
    pub total_capacity_hours: f64, // Total watt-hours delivered
}

/// Registry of all bonded printers
pub struct PrinterRegistry {
    printers: Arc<RwLock<HashMap<String, BondedPrinter>>>,
    storage_path: String,
}

impl PrinterRegistry {
    pub fn new() -> Self {
        Self {
            printers: Arc::new(RwLock::new(HashMap::new())),
            storage_path: "printer_registry.json".to_string(),
        }
    }
    
    pub fn with_path(path: String) -> Self {
        Self {
            printers: Arc::new(RwLock::new(HashMap::new())),
            storage_path: path,
        }
    }
    
    /// Load printer registry from disk
    pub async fn load(&self) -> Result<(), String> {
        if !Path::new(&self.storage_path).exists() {
            info!("📂 No existing printer registry found at {}", self.storage_path);
            return Ok(());
        }
        
        let contents = std::fs::read_to_string(&self.storage_path)
            .map_err(|e| format!("Failed to read printer registry: {}", e))?;
        
        let loaded: HashMap<String, BondedPrinter> = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse printer registry: {}", e))?;
        
        let mut printers = self.printers.write().await;
        *printers = loaded;
        
        info!("✅ Loaded {} printer(s) from {}", printers.len(), self.storage_path);
        Ok(())
    }
    
    /// Save printer registry to disk
    async fn save(&self) -> Result<(), String> {
        let printers = self.printers.read().await;
        
        let json = serde_json::to_string_pretty(&*printers)
            .map_err(|e| format!("Failed to serialize printer registry: {}", e))?;
        
        std::fs::write(&self.storage_path, json)
            .map_err(|e| format!("Failed to write printer registry: {}", e))?;
        
        Ok(())
    }

    /// Register a new printer and return certificate
    pub async fn register(
        &self,
        registration: PrinterRegistration,
        certificate: PrinterCertificate,
    ) -> Result<(), String> {
        let mut printers = self.printers.write().await;
        
        let bonded = BondedPrinter {
            registration: registration.clone(),
            certificate,
            assigned_contract: None,
            prints_completed: 0,
            total_capacity_hours: 0.0,
        };

        printers.insert(registration.printer_id.clone(), bonded);
        drop(printers); // Release lock before save
        
        self.save().await.ok(); // Persist to disk
        Ok(())
    }

    /// Get printer by ID
    pub async fn get(&self, printer_id: &str) -> Option<BondedPrinter> {
        let printers = self.printers.read().await;
        printers.get(printer_id).cloned()
    }

    /// Assign contract to printer
    /// Allows reassignment - http_api checks if contract is already completed before calling
    pub async fn assign_contract(
        &self,
        printer_id: &str,
        contract_id: String,
    ) -> Result<(), String> {
        let mut printers = self.printers.write().await;
        
        let printer = printers
            .get_mut(printer_id)
            .ok_or_else(|| format!("Printer {} not found", printer_id))?;

        // Allow reassignment - just overwrite any existing assignment
        printer.assigned_contract = Some(contract_id);
        drop(printers); // Release lock before save
        
        self.save().await.ok(); // Persist to disk
        Ok(())
    }

    /// Mark print completed and update stats
    pub async fn complete_print(
        &self,
        printer_id: &str,
        capacity_hours: f64,
    ) -> Result<(), String> {
        let mut printers = self.printers.write().await;
        
        let printer = printers
            .get_mut(printer_id)
            .ok_or_else(|| format!("Printer {} not found", printer_id))?;

        printer.prints_completed += 1;
        printer.total_capacity_hours += capacity_hours;
        printer.assigned_contract = None; // Clear assignment
        drop(printers); // Release lock before save
        
        self.save().await.ok(); // Persist to disk
        Ok(())
    }

    /// Get all registered printers
    pub async fn list_all(&self) -> Vec<BondedPrinter> {
        let printers = self.printers.read().await;
        printers.values().cloned().collect()
    }

    /// Validate printer model (basic validation for demo)
    pub fn validate_model(model: &str) -> bool {
        // For demo, accept common 3D printer models
        let valid_models = vec![
            "Ender3V3KE",
            "Ender3",
            "Ender3Pro",
            "Ender3S1",
            "Ender5",
            "Prusa MK3S",
            "Prusa MK4",
            "Bambu X1",
            "Bambu P1P",
        ];

        valid_models.iter().any(|m| model.contains(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_printer() {
        let registry = PrinterRegistry::new();

        let registration = PrinterRegistration {
            printer_id: "test-printer-001".to_string(),
            model: "Ender3V3KE".to_string(),
            manufacturer: "Creality".to_string(),
            serial_number: "TEST123".to_string(),
            rated_watts: 350,
            public_key: "deadbeef".to_string(),
        };

        let cert = PrinterCertificate {
            printer_id: "test-printer-001".to_string(),
            model: "Ender3V3KE".to_string(),
            rated_watts: 350,
            issued_at: "2025-11-19T00:00:00Z".to_string(),
            expires_at: None,
            public_key: "deadbeef".to_string(),
            certificate_hash: "abc123".to_string(),
            signature: "signature".to_string(),
        };

        registry.register(registration, cert).await.unwrap();

        let printer = registry.get("test-printer-001").await.unwrap();
        assert_eq!(printer.registration.printer_id, "test-printer-001");
    }

    #[tokio::test]
    async fn test_assign_contract() {
        let registry = PrinterRegistry::new();

        let registration = PrinterRegistration {
            printer_id: "test-printer-001".to_string(),
            model: "Ender3V3KE".to_string(),
            manufacturer: "Creality".to_string(),
            serial_number: "TEST123".to_string(),
            rated_watts: 350,
            public_key: "deadbeef".to_string(),
        };

        let cert = PrinterCertificate {
            printer_id: "test-printer-001".to_string(),
            model: "Ender3V3KE".to_string(),
            rated_watts: 350,
            issued_at: "2025-11-19T00:00:00Z".to_string(),
            expires_at: None,
            public_key: "deadbeef".to_string(),
            certificate_hash: "abc123".to_string(),
            signature: "signature".to_string(),
        };

        registry.register(registration, cert).await.unwrap();
        
        registry
            .assign_contract("test-printer-001", "contract-123".to_string())
            .await
            .unwrap();

        let printer = registry.get("test-printer-001").await.unwrap();
        assert_eq!(printer.assigned_contract, Some("contract-123".to_string()));
    }

    #[test]
    fn test_validate_model() {
        assert!(PrinterRegistry::validate_model("Ender3V3KE"));
        assert!(PrinterRegistry::validate_model("Prusa MK3S"));
        assert!(!PrinterRegistry::validate_model("UnknownModel"));
    }
}
