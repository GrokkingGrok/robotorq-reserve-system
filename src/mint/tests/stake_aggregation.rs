//! Tests stake aggregation from ingots into certificate and batch.
use std::sync::Arc;
use std::time::SystemTime;
use mint::engine::{proof_engine::ProofEngine, batcher::Batcher};
use mint::config::MintConfig;
use mint::metrics::MintMetrics;
use mint::models::token_torq_ingot::TokenTorqIngot;
use tokio::sync::mpsc;

fn make_ingot(id: usize, stake: i64, joules: i64) -> TokenTorqIngot {
    TokenTorqIngot {
        ingot_id: format!("ingot-{}", id),
        contract_id: "contract-A".into(),
        joules_total: joules,
        robostake_total_jouletorq: stake,
        unit_count: joules as u32,
        merkle_root: "a".repeat(64),
        unit_hashes: vec![],
        timestamp: SystemTime::now(),
        signature: vec![],
    }
}

#[test]
fn single_ingot_stake_propagates() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    let ingot = make_ingot(0, 124_534, 3600);
    let (cert, _proof) = engine.create_certificate_and_proof_sync(vec![ingot]).expect("create cert");
    assert_eq!(cert.total_stake_jouletorq, 124_534, "stake should equal ingot stake");
}

#[test]
fn multi_ingot_stake_sums() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    let ingots: Vec<_> = (0..5).map(|i| make_ingot(i, 10_000 + i as i64, 3600)).collect(); // stakes: 10000..10004
    let expected: i64 = ingots.iter().map(|g| g.robostake_total_jouletorq).sum();
    let (cert, _proof) = engine.create_certificate_and_proof_sync(ingots).expect("create cert");
    assert_eq!(cert.total_stake_jouletorq, expected);
}

#[tokio::test]
async fn async_multi_ingot_stake_sums() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    let ingots: Vec<_> = (0..4).map(|i| make_ingot(i, 20_000 + i as i64, 3600)).collect();
    let expected: i64 = ingots.iter().map(|g| g.robostake_total_jouletorq).sum();
    let (cert, _proof) = engine.create_certificate_and_proof(ingots).await.expect("create cert async");
    assert_eq!(cert.total_stake_jouletorq, expected);
}

#[test]
fn stake_overflow_rejected_sync() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    // Two ingots with stakes crafted to overflow i64 when summed.
    let half_plus = i64::MAX / 2 + 1; // ensures 2 * half_plus > i64::MAX
    let ingot_a = make_ingot(0, half_plus, 3600);
    let ingot_b = make_ingot(1, half_plus, 3600);
    let res = engine.create_certificate_and_proof_sync(vec![ingot_a, ingot_b]);
    assert!(res.is_err(), "overflow stake should be rejected");
    let err_msg = format!("{}", res.err().unwrap());
    assert!(err_msg.contains("stake overflow") || err_msg.contains("negative stake"));
}

#[tokio::test]
async fn stake_overflow_rejected_async() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    let half_plus = i64::MAX / 2 + 1;
    let ingot_a = make_ingot(0, half_plus, 3600);
    let ingot_b = make_ingot(1, half_plus, 3600);
    let res = engine.create_certificate_and_proof(vec![ingot_a, ingot_b]).await;
    assert!(res.is_err(), "overflow stake should be rejected (async)");
    let err_msg = format!("{}", res.err().unwrap());
    assert!(err_msg.contains("stake overflow") || err_msg.contains("negative stake"));
}

#[tokio::test]
async fn batcher_does_not_mutate_stake() {
    let cfg = Arc::new(MintConfig::default());
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));
    // Simulate two certificates each from one ingot with distinct stake values
    let (cert1, _) = engine.create_certificate_and_proof_sync(vec![make_ingot(0, 50_000, 3600)]).unwrap();
    let (cert2, _) = engine.create_certificate_and_proof_sync(vec![make_ingot(1, 74_534, 3600)]).unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    let batcher = Batcher::new(tx, 60, Arc::clone(&metrics));
    batcher.add_certificate(cert1.clone()).await.unwrap();
    batcher.add_certificate(cert2.clone()).await.unwrap();
    // Force flush by hitting target size (we added only 2; target is 100) -> need manual flush path exposed
    // Instead create a small batch directly using create_batch
    let batch = batcher.create_batch(vec![cert1.clone(), cert2.clone()]).await.unwrap();
    assert_eq!(batch.certificates.len(), 2);
    assert_eq!(batch.cert_count, 2);
    let stakes: Vec<i64> = batch.certificates.iter().map(|c| c.total_stake_jouletorq).collect();
    assert!(stakes.contains(&50_000) && stakes.contains(&74_534));
    let stake_sum = stakes.iter().sum::<i64>();
    assert_eq!(stake_sum, 124_534);
    assert_eq!(stake_sum, batch.total_robostake, "batch stake must equal sum of certificate stakes");
    let joule_sum: i64 = batch.certificates.iter().map(|c| c.total_jouletorq).sum();
    assert_eq!(joule_sum, batch.canonical_total_jouletorq, "batch joule total must equal sum of certificate joules");
}
