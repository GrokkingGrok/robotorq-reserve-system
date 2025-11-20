use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub printer_id: String,
    pub model: String,
    pub rated_watts: u32,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub public_key: String,
    pub certificate_hash: String,
    pub signature: String,
}

pub struct CertificateManager {
    printer_id: String,
    certificate: Option<Certificate>,
}

impl CertificateManager {
    pub fn new(printer_id: &str) -> Self {
        Self {
            printer_id: printer_id.to_string(),
            certificate: None,
        }
    }
    
    pub fn certificate(&self) -> Option<&Certificate> {
        self.certificate.as_ref()
    }
    
    pub fn load_certificate(&mut self) -> Result<()> {
        let path = self.certificate_path();
        let contents = fs::read_to_string(&path)?;
        let cert: Certificate = serde_json::from_str(&contents)?;
        
        // TODO: Verify certificate signature with Mint public key
        
        self.certificate = Some(cert);
        Ok(())
    }
    
    pub fn save_certificate(&mut self, cert: &Certificate) -> Result<()> {
        // Ensure data directory exists
        fs::create_dir_all("./data")?;
        
        let path = self.certificate_path();
        let contents = serde_json::to_string_pretty(cert)?;
        fs::write(&path, contents)?;
        
        self.certificate = Some(cert.clone());
        Ok(())
    }
    
    pub fn is_valid(&self) -> bool {
        if let Some(cert) = &self.certificate {
            // TODO: Check expiration date
            // TODO: Verify signature
            cert.printer_id == self.printer_id
        } else {
            false
        }
    }
    
    fn certificate_path(&self) -> PathBuf {
        PathBuf::from(format!("./data/{}_certificate.json", self.printer_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_certificate_manager_creation() {
        let manager = CertificateManager::new("test-printer-001");
        assert_eq!(manager.printer_id, "test-printer-001");
        assert!(manager.certificate().is_none());
    }
    
    #[test]
    fn test_certificate_path() {
        let manager = CertificateManager::new("test-printer-001");
        let path = manager.certificate_path();
        assert_eq!(path, PathBuf::from("./data/test-printer-001_certificate.json"));
    }
}
