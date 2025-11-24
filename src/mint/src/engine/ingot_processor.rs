use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use crate::models::{token_torq_ingot::TokenTorqIngot, robotorq_certificate::RoboTorqCertificate, robo_torq_proof::RoboTorqProof};
use crate::metrics::MintMetrics;
use crate::engine::proof_engine::ProofEngine;

/// IngotProcessor: Buffers ingots and creates certificates when 1000 are received.
pub struct IngotProcessor {
    ingot_buffer: Mutex<VecDeque<TokenTorqIngot>>,
    certificate_sender: mpsc::Sender<RoboTorqCertificate>,
    proof_engine: Arc<ProofEngine>,
    metrics: Arc<MintMetrics>,
}

impl IngotProcessor {
    pub fn new(
        certificate_sender: mpsc::Sender<RoboTorqCertificate>,
        proof_engine: Arc<ProofEngine>,
        metrics: Arc<MintMetrics>,
    ) -> Self {
        Self {
            ingot_buffer: Mutex::new(VecDeque::with_capacity(1024)), // Pre-allocate capacity
            certificate_sender,
            proof_engine,
            metrics,
        }
    }

    /// Add an ingot to the buffer. If we reach 1000, create a certificate.
    /// This method is now async to allow for non-blocking certificate sending.
    pub async fn process_ingot_async(&self, ingot: TokenTorqIngot) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let should_create_cert = {
            let mut buffer = self.ingot_buffer.lock().unwrap();
            buffer.push_back(ingot);

            // Check if we have enough ingots for a certificate
            let len = buffer.len();
            self.metrics.set_ingot_buffer_size(len as i64);

            len >= 1000
        };

        if should_create_cert {
            self.create_and_send_certificate().await?;
        }

        Ok(())
    }

    /// Synchronous version for backward compatibility (used in tests)
    pub fn process_ingot(&self, ingot: TokenTorqIngot) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let should_create_cert = {
            let mut buffer = self.ingot_buffer.lock().unwrap();
            buffer.push_back(ingot);

            // Check if we have enough ingots for a certificate
            let len = buffer.len();
            self.metrics.set_ingot_buffer_size(len as i64);

            len >= 1000
        };

        if should_create_cert {
            // For sync version, we still use try_send but log if it fails
            let ingots = {
                let mut buffer = self.ingot_buffer.lock().unwrap();
                if buffer.len() < 1000 {
                    return Ok(()); // Not enough ingots
                }
                // Extract exactly 1000 ingots
                let ingots: Vec<TokenTorqIngot> = (0..1000).filter_map(|_| buffer.pop_front()).collect();
                self.metrics.set_ingot_buffer_size(buffer.len() as i64);
                ingots
            };

            let (certificate, _proof) = self.create_certificate_sync(ingots)?;
            self.certificate_sender.try_send(certificate).map_err(|e| format!("Failed to send certificate: {}", e))?;
        }

        Ok(())
    }

    /// Extract 1000 ingots and create/send a certificate
    async fn create_and_send_certificate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ingots = {
            let mut buffer = self.ingot_buffer.lock().unwrap();
            if buffer.len() < 1000 {
                return Ok(()); // Not enough ingots
            }

            // Extract exactly 1000 ingots
            let ingots: Vec<TokenTorqIngot> = (0..1000).filter_map(|_| buffer.pop_front()).collect();
            self.metrics.set_ingot_buffer_size(buffer.len() as i64);
            ingots
        };

        // Create certificate outside the lock
        let (certificate, _proof) = self.create_certificate_async(ingots).await?;

        // Send certificate asynchronously
        self.certificate_sender.send(certificate).await?;

        Ok(())
    }

    /// Async version of certificate creation
    async fn create_certificate_async(
        &self,
        ingots: Vec<TokenTorqIngot>,
    ) -> Result<(RoboTorqCertificate, RoboTorqProof), Box<dyn std::error::Error + Send + Sync>> {
        // Use proof engine to create certificate and proof
        self.proof_engine.create_certificate_and_proof(ingots).await
    }

    /// Synchronous version for backward compatibility
    fn create_certificate_sync(
        &self,
        ingots: Vec<TokenTorqIngot>,
    ) -> Result<(RoboTorqCertificate, RoboTorqProof), Box<dyn std::error::Error + Send + Sync>> {
        // Use proof engine to create certificate and proof synchronously
        self.proof_engine.create_certificate_and_proof_sync(ingots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use std::sync::Arc;
    use std::time::SystemTime;
    use crate::config::MintConfig;

    fn create_test_ingot(id: &str, contract: &str) -> TokenTorqIngot {
        TokenTorqIngot {
            ingot_id: id.to_string(),
            contract_id: contract.to_string(),
            joules_total: 1000.0,
            robostake_total_micro_rt: 50000,
            unit_count: 3600,
            merkle_root: "test_root".to_string(),
            unit_hashes: vec!["hash1".to_string(), "hash2".to_string()],
            timestamp: SystemTime::now(),
            signature: vec![1, 2, 3],
        }
    }

    #[test]
    fn test_ingot_processor_buffer_under_threshold() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(MintMetrics::new());
        let config = Arc::new(MintConfig::default());
        let proof_engine = Arc::new(ProofEngine::new(config, Arc::clone(&metrics)));
        let processor = IngotProcessor::new(tx, proof_engine, Arc::clone(&metrics));

        // Add 999 ingots (under threshold)
        for i in 0..999 {
            let ingot = create_test_ingot(&format!("ingot-{}", i), "contract-1");
            processor.process_ingot(ingot).unwrap();
        }

        // Check that no certificate was sent
        assert!(rx.try_recv().is_err());

        // Check buffer size metric
        assert_eq!(metrics.ingot_buffer_size.get(), 999);
    }

    #[test]
    fn test_ingot_processor_certificate_creation() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(MintMetrics::new());
        let config = Arc::new(MintConfig::default());
        let proof_engine = Arc::new(ProofEngine::new(config, Arc::clone(&metrics)));
        let processor = IngotProcessor::new(tx, proof_engine, Arc::clone(&metrics));

        // Add exactly 1000 ingots
        for i in 0..1000 {
            let ingot = create_test_ingot(&format!("ingot-{}", i), "contract-1");
            processor.process_ingot(ingot).unwrap();
        }

        // Check that a certificate was sent
        let cert = rx.try_recv().unwrap();

        // Verify certificate properties
        assert!(cert.cert_id.starts_with("cert-"));
        assert_eq!(cert.contract_ids.len(), 1);
        assert_eq!(cert.contract_ids[0], "contract-1");

        // Check metrics
        assert_eq!(metrics.certificates_created_total.get(), 1);
        assert_eq!(metrics.ingot_buffer_size.get(), 0); // Buffer should be cleared
    }

    #[test]
    fn test_ingot_processor_multiple_contracts() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(MintMetrics::new());
        let config = Arc::new(MintConfig::default());
        let proof_engine = Arc::new(ProofEngine::new(config, Arc::clone(&metrics)));
        let processor = IngotProcessor::new(tx, proof_engine, Arc::clone(&metrics));

        // Add ingots from multiple contracts
        for i in 0..500 {
            let ingot = create_test_ingot(&format!("ingot-a-{}", i), "contract-a");
            processor.process_ingot(ingot).unwrap();
        }
        for i in 0..500 {
            let ingot = create_test_ingot(&format!("ingot-b-{}", i), "contract-b");
            processor.process_ingot(ingot).unwrap();
        }

        // Check that a certificate was sent
        let cert = rx.try_recv().unwrap();

        // Should have both contracts
        assert_eq!(cert.contract_ids.len(), 2);
        assert!(cert.contract_ids.contains(&"contract-a".to_string()));
        assert!(cert.contract_ids.contains(&"contract-b".to_string()));
    }

    #[test]
    fn test_ingot_processor_multiple_certificates() {
        let (tx, mut rx) = mpsc::channel(10);
        let metrics = Arc::new(MintMetrics::new());
        let config = Arc::new(MintConfig::default());
        let proof_engine = Arc::new(ProofEngine::new(config, Arc::clone(&metrics)));
        let processor = IngotProcessor::new(tx, proof_engine, Arc::clone(&metrics));

        // Add 2000 ingots (should create 2 certificates)
        for i in 0..2000 {
            let ingot = create_test_ingot(&format!("ingot-{}", i), "contract-1");
            processor.process_ingot(ingot).unwrap();
        }

        // Check that two certificates were sent
        let cert1 = rx.try_recv().unwrap();
        let cert2 = rx.try_recv().unwrap();

        assert_ne!(cert1.cert_id, cert2.cert_id);

        // Check metrics
        assert_eq!(metrics.certificates_created_total.get(), 2);
        assert_eq!(metrics.ingot_buffer_size.get(), 0);
    }
}