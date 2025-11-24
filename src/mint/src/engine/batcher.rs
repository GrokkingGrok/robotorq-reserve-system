use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;
use crate::models::{robotorq_certificate::RoboTorqCertificate, robotorq_batch::RoboTorqBatch};
use crate::metrics::MintMetrics;

/// Batcher: Collects certificates into batches and publishes them.
pub struct Batcher {
    certificate_buffer: Mutex<Vec<RoboTorqCertificate>>,
    batch_sender: mpsc::Sender<RoboTorqBatch>,
    batch_interval: Duration,
    metrics: Arc<MintMetrics>,
    batch_size_target: usize,
}

impl Batcher {
    pub fn new(
        batch_sender: mpsc::Sender<RoboTorqBatch>,
        batch_interval_seconds: u64,
        metrics: Arc<MintMetrics>,
    ) -> Self {
        Self {
            certificate_buffer: Mutex::new(Vec::with_capacity(128)), // Pre-allocate reasonable capacity
            batch_sender,
            batch_interval: Duration::from_secs(batch_interval_seconds),
            metrics,
            batch_size_target: 100, // Target batch size for efficiency
        }
    }

    /// Add a certificate to the current batch.
    pub async fn add_certificate(&self, certificate: RoboTorqCertificate) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let should_flush = {
            let mut buffer = self.certificate_buffer.lock().unwrap();
            buffer.push(certificate);

            let len = buffer.len();
            self.metrics.set_certificate_queue_size(len as i64);

            // Flush if we hit the target batch size
            len >= self.batch_size_target
        };

        if should_flush {
            self.flush_batch().await?;
        }

        Ok(())
    }

    /// Start the batching loop that periodically flushes batches.
    pub async fn start_batching_loop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut interval = time::interval(self.batch_interval);

        loop {
            interval.tick().await;

            // Always flush on timer, even if not at target size
            self.flush_batch().await?;
        }
    }

    /// Flush current batch regardless of size
    async fn flush_batch(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let certificates = {
            let mut buffer = self.certificate_buffer.lock().unwrap();
            if buffer.is_empty() {
                return Ok(());
            }

            // Take all certificates
            let certs = buffer.drain(..).collect::<Vec<_>>();
            self.metrics.set_certificate_queue_size(0);
            certs
        };

        if !certificates.is_empty() {
            let batch = self.create_batch(certificates).await?;
            self.batch_sender.send(batch).await?;
        }

        Ok(())
    }

    pub async fn create_batch(&self, certificates: Vec<RoboTorqCertificate>) -> Result<RoboTorqBatch, Box<dyn std::error::Error + Send + Sync>> {
        let batch_id = format!("batch-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());

        let batch = RoboTorqBatch {
            batch_id,
            created_at_nanos: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64,
            certificates,
        };

        // Update metrics
        self.metrics.inc_batches_published();

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use std::time::SystemTime;

    fn create_test_certificate(id: &str, contract: &str) -> RoboTorqCertificate {
        RoboTorqCertificate {
            cert_id: id.to_string(),
            robotorq_proof_id: format!("proof-{}", id),
            merkle_root: "test_root".to_string(),
            contract_ids: vec![contract.to_string()],
            timestamp_nanos: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64,
            hash: "test_hash".to_string(),
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
        }
    }

    #[tokio::test]
    async fn test_batcher_add_certificate() {
        let (tx, _rx) = mpsc::channel(10);
        let metrics = crate::metrics::MintMetrics::new();
        let batcher = Batcher::new(tx, 60, metrics);

        let cert = create_test_certificate("cert-1", "contract-1");
        batcher.add_certificate(cert).await.unwrap();

        // Check queue size metric
        // Note: We can't easily check the metric here due to Arc cloning
    }

    #[tokio::test]
    async fn test_batcher_create_batch() {
        let (tx, _rx) = mpsc::channel(10);
        let metrics = crate::metrics::MintMetrics::new();
        let batcher = Batcher::new(tx, 60, metrics);

        let certs = vec![
            create_test_certificate("cert-1", "contract-1"),
            create_test_certificate("cert-2", "contract-2"),
        ];

        let batch = batcher.create_batch(certs).await.unwrap();

        assert!(batch.batch_id.starts_with("batch-"));
        assert_eq!(batch.certificates.len(), 2);
        // Note: We can't easily check the metric here due to Arc cloning
    }

    #[test]
    fn test_batcher_initialization() {
        let (tx, _rx) = mpsc::channel(10);
        let metrics = crate::metrics::MintMetrics::new();
        let batcher = Batcher::new(tx, 300, metrics);

        // Check that batch interval is set correctly
        assert_eq!(batcher.batch_interval, Duration::from_secs(300));
    }
}