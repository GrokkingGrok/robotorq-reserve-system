use prometheus::{IntCounter, IntGauge, Histogram, HistogramOpts, Registry, Encoder, TextEncoder};
use std::sync::Arc;

#[derive(Clone)]
pub struct MintMetrics {
    pub registry: Registry,
    pub ingots_received_total: IntCounter,
    pub certificates_created_total: IntCounter,
    pub batches_published_total: IntCounter,
    pub proofs_created_total: IntCounter,
    pub signatures_created_total: IntCounter,
    #[allow(dead_code)]
    pub stake_accumulated_total: IntGauge,
    pub ingot_buffer_size: IntGauge,
    pub certificate_queue_size: IntGauge,
    #[allow(dead_code)]
    pub batch_processing_time: Histogram,
    #[allow(dead_code)]
    pub proof_creation_time: Histogram,
    #[allow(dead_code)]
    pub merkle_operations_total: IntCounter,
    pub nats_publish_errors_total: IntCounter,
    pub processing_errors_total: IntCounter,
}

impl MintMetrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();

        let ingots_received_total = IntCounter::new("mint_ingots_received_total", "Total ingots received from Refinery").unwrap();
        let certificates_created_total = IntCounter::new("mint_certificates_created_total", "Total RoboTorq certificates created").unwrap();
        let batches_published_total = IntCounter::new("mint_batches_published_total", "Total RoboTorq batches published to Vault").unwrap();
        let proofs_created_total = IntCounter::new("mint_proofs_created_total", "Total RoboTorq proofs created").unwrap();
        let signatures_created_total = IntCounter::new("mint_signatures_created_total", "Total cryptographic signatures created").unwrap();
        let stake_accumulated_total = IntGauge::new("mint_stake_accumulated_total", "Current total stake accumulated for batching").unwrap();
        let ingot_buffer_size = IntGauge::new("mint_ingot_buffer_size", "Current number of ingots in buffer").unwrap();
        let certificate_queue_size = IntGauge::new("mint_certificate_queue_size", "Current number of certificates queued for batching").unwrap();
        let batch_processing_time = Histogram::with_opts(HistogramOpts::new("mint_batch_processing_time_seconds", "Time spent processing batches")).unwrap();
        let proof_creation_time = Histogram::with_opts(HistogramOpts::new("mint_proof_creation_time_seconds", "Time spent creating proofs")).unwrap();
        let merkle_operations_total = IntCounter::new("mint_merkle_operations_total", "Total merkle tree operations performed").unwrap();
        let nats_publish_errors_total = IntCounter::new("mint_nats_publish_errors_total", "Total NATS publish errors").unwrap();
        let processing_errors_total = IntCounter::new("mint_processing_errors_total", "Total processing errors").unwrap();

        registry.register(Box::new(ingots_received_total.clone())).unwrap();
        registry.register(Box::new(certificates_created_total.clone())).unwrap();
        registry.register(Box::new(batches_published_total.clone())).unwrap();
        registry.register(Box::new(proofs_created_total.clone())).unwrap();
        registry.register(Box::new(signatures_created_total.clone())).unwrap();
        registry.register(Box::new(stake_accumulated_total.clone())).unwrap();
        registry.register(Box::new(ingot_buffer_size.clone())).unwrap();
        registry.register(Box::new(certificate_queue_size.clone())).unwrap();
        registry.register(Box::new(batch_processing_time.clone())).unwrap();
        registry.register(Box::new(proof_creation_time.clone())).unwrap();
        registry.register(Box::new(merkle_operations_total.clone())).unwrap();
        registry.register(Box::new(nats_publish_errors_total.clone())).unwrap();
        registry.register(Box::new(processing_errors_total.clone())).unwrap();

        Arc::new(Self {
            registry,
            ingots_received_total,
            certificates_created_total,
            batches_published_total,
            proofs_created_total,
            signatures_created_total,
            stake_accumulated_total,
            ingot_buffer_size,
            certificate_queue_size,
            batch_processing_time,
            proof_creation_time,
            merkle_operations_total,
            nats_publish_errors_total,
            processing_errors_total,
        })
    }

    pub fn encode(&self) -> String {
        let mf = self.registry.gather();
        let mut buf = Vec::new();
        TextEncoder::new().encode(&mf, &mut buf).unwrap();
        String::from_utf8(buf).unwrap_or_default()
    }

    // Convenience methods for updating metrics
    pub fn inc_ingots_received(&self) { self.ingots_received_total.inc(); }
    pub fn inc_certificates_created(&self) { self.certificates_created_total.inc(); }
    pub fn inc_batches_published(&self) { self.batches_published_total.inc(); }
    pub fn inc_proofs_created(&self) { self.proofs_created_total.inc(); }
    pub fn inc_signatures_created(&self, count: i64) { self.signatures_created_total.inc_by(count as u64); }
    #[allow(dead_code)]
    pub fn set_stake_accumulated(&self, val: i64) { self.stake_accumulated_total.set(val); }
    pub fn set_ingot_buffer_size(&self, val: i64) { self.ingot_buffer_size.set(val); }
    pub fn set_certificate_queue_size(&self, val: i64) { self.certificate_queue_size.set(val); }
    #[allow(dead_code)]
    pub fn observe_batch_processing_time(&self, duration: f64) { self.batch_processing_time.observe(duration); }
    #[allow(dead_code)]
    pub fn observe_proof_creation_time(&self, duration: f64) { self.proof_creation_time.observe(duration); }
    #[allow(dead_code)]
    pub fn inc_merkle_operations(&self) { self.merkle_operations_total.inc(); }
    pub fn inc_nats_publish_errors(&self) { self.nats_publish_errors_total.inc(); }
    pub fn inc_processing_errors(&self) { self.processing_errors_total.inc(); }
}