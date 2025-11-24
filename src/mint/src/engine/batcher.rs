use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time;
use crate::models::{robotorq_certificate::RoboTorqCertificate, robotorq_batch::RoboTorqBatch};
// Removed jouletorq_to_triple import; test helper builds triple directly.
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
        let stake_sum_i128: i128 = certificates.iter().map(|c| c.total_stake_jouletorq as i128).sum();
        let joule_sum_i128: i128 = certificates.iter().map(|c| c.total_jouletorq as i128).sum();
        if stake_sum_i128 < 0 || joule_sum_i128 < 0 { return Err("negative economic totals".into()); }
        if stake_sum_i128 > i64::MAX as i128 || joule_sum_i128 > i64::MAX as i128 { return Err("economic total overflow".into()); }
        let created_at_nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as i64;
        let batch = RoboTorqBatch {
            event_type: "robotorqcert_batch_completed".to_string(),
            batch_id,
            created_at_nanos,
            cert_count: certificates.len(),
            total_robostake: stake_sum_i128 as i64,
            canonical_total_jouletorq: joule_sum_i128 as i64,
            certificates,
        };
        batch.validate().map_err(|e| format!("batch validation failed: {}", e))?;
        self.metrics.inc_batches_published();
        self.metrics.set_last_batch(batch.total_robostake, batch.canonical_total_jouletorq);
        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use std::time::SystemTime;

    fn create_test_certificate(id: &str, contract: &str) -> RoboTorqCertificate {
        let c = RoboTorqCertificate {
            cert_id: id.to_string(),
            robotorq_proof_id: format!("proof-{}", id),
            merkle_root: "test_root".to_string(),
            contract_ids: vec![contract.to_string()],
            timestamp_nanos: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64,
            hash: "test_hash".to_string(),
            status: crate::models::robotorq_certificate::CertStatus::Digital,
            bearer_bond_id: None,
            total_jouletorq: 3600 * 1000,
            total_stake_jouletorq: 3600 * 500,
            total_triple: common::triples::jouletorq_to_triple(3600 * 1000),
        };
        c
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
        assert_eq!(batch.cert_count, 2);
        let stake_sum: i64 = batch.certificates.iter().map(|c| c.total_stake_jouletorq).sum();
        assert_eq!(stake_sum, batch.total_robostake);
        let joule_sum: i64 = batch.certificates.iter().map(|c| c.total_jouletorq).sum();
        assert_eq!(joule_sum, batch.canonical_total_jouletorq);
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