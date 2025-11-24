//! Integration test: Exercise batcher end-to-end without NATS.
//! Simulates certificates arriving and verifies batch flush on size & timer.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::mpsc;
use mint::engine::batcher::Batcher;
use mint::models::robotorq_certificate::{RoboTorqCertificate, CertStatus};
use mint::metrics::MintMetrics;
use common::triples::jouletorq_to_triple;

fn make_cert(idx: usize) -> RoboTorqCertificate {
    let total = 3600 * 1000; // reuse constant size
    RoboTorqCertificate {
        cert_id: format!("cert-{}", idx),
        robotorq_proof_id: format!("proof-cert-{}", idx),
        merkle_root: "r".repeat(64),
        contract_ids: vec!["contract-A".into()],
        timestamp_nanos: 123,
        hash: "h".repeat(64),
        status: CertStatus::Digital,
        bearer_bond_id: None,
        total_jouletorq: total,
        total_stake_jouletorq: 3600 * 500, // arbitrary stake for test fixture
        total_triple: jouletorq_to_triple(total),
    }
}

#[tokio::test]
async fn batch_flushes_on_target_size() {
    let (tx, mut rx) = mpsc::channel(10);
    let metrics = MintMetrics::new();
    // Use very long interval so only size triggers flush
    let batcher = Batcher::new(tx, 10_000, Arc::clone(&metrics));

    for i in 0..100 { // equals batch_size_target
        batcher.add_certificate(make_cert(i)).await.unwrap();
    }

    // Expect one batch emitted
    let batch = rx.recv().await.expect("expected batch");
    assert_eq!(batch.certificates.len(), 100, "batch should contain 100 certificates");
}

#[tokio::test]
async fn batch_flushes_on_interval() {
    let (tx, mut rx) = mpsc::channel(10);
    let metrics = MintMetrics::new();
    // Short interval to force time-based flush
    let batcher = Arc::new(Batcher::new(tx, 1, Arc::clone(&metrics))); // 1 second

    // Spawn batching loop
    let b_clone = Arc::clone(&batcher);
    tokio::spawn(async move { b_clone.start_batching_loop().await.unwrap(); });

    // Add fewer than target size
    for i in 0..5 {
        batcher.add_certificate(make_cert(i)).await.unwrap();
    }

    // Wait for interval tick & flush
    sleep(Duration::from_secs(2)).await; // ensure at least one tick

    let batch = rx.recv().await.expect("expected interval batch");
    assert_eq!(batch.certificates.len(), 5, "interval flush should include 5 certificates");
}
