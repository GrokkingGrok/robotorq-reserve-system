//! Ledger reconciliation tests for RoboTorqCertificate economic fields.
//! Ensures integer total_jouletorq matches ingot sum and Triple decomposition.

use std::time::{SystemTime};
use mint::engine::proof_engine::ProofEngine; // assuming re-export in lib.rs
use mint::models::token_torq_ingot::TokenTorqIngot;
use mint::config::MintConfig;
use mint::metrics::MintMetrics;
use common::triples::{Triple, triple_to_jouletorq, JOULETORQ_PER_ROBOTORQ};
use std::sync::Arc;

fn dummy_ingot(id: usize, joules: i64) -> TokenTorqIngot {
    let units = joules as u32;
    TokenTorqIngot {
        ingot_id: format!("ingot-{}", id),
        contract_id: "contract-A".into(),
        joules_total: joules,
        robostake_total_jouletorq: joules, // stake == joules for test
        unit_count: units,
        merkle_root: "f".repeat(64),
        unit_hashes: vec!["e".repeat(64); units as usize],
        timestamp: SystemTime::now(),
        signature: vec![],
    }
}

fn test_engine() -> ProofEngine {
    let cfg = Arc::new(MintConfig {
        nats_url: "nats://test".into(),
        ingots_per_cert: 1000,
        batch_threshold_count: 10,
        batch_threshold_seconds: 300,
        proof_interval_count: 100,
        proof_interval_seconds: 3600,
        merkle_tree_depth: 16,
        enable_crypto: false,
        signature_algorithm: "falcon1024".into(),
        min_stake_micro_rt: 50_000,
        key_storage_path: None,
        enable_archive: true,
    });
    let metrics = Arc::new(MintMetrics::new());
    ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics))
}

#[test]
fn certificate_total_matches_ingot_sum_and_triple() {
    let engine = test_engine();
    // Create 5 ingots of 3600 joules each (one partial RoboTorq)
    let ingots: Vec<_> = (0..5).map(|i| dummy_ingot(i, 3600)).collect();
    let total_expected: i64 = ingots.iter().map(|g| g.joules_total).sum();

    let (cert, _proof) = engine.create_certificate_and_proof_sync(ingots).expect("certificate creation");

    assert_eq!(cert.total_jouletorq, total_expected, "certificate total_jouletorq should equal sum of ingot joules");

    // Reconstruct from triple
    let reconstructed = triple_to_jouletorq(cert.total_triple);
    assert_eq!(reconstructed, cert.total_jouletorq, "Triple decomposition must reconstruct total_jouletorq exactly");

    // Fractional RoboTorq check
    let fractional_rt = cert.total_jouletorq as f64 / JOULETORQ_PER_ROBOTORQ as f64;
    assert!(fractional_rt < 1.0, "Expected less than one RoboTorq for 5 ingots (5*3600 < RoboTorq)");
}

#[test]
fn triple_remainders_invariant_bounds() {
    let engine = test_engine();
    // 1001 ingots: ensures at least one full robotorq token remainder scenario.
    let ingots: Vec<_> = (0..1001).map(|i| dummy_ingot(i, 3600)).collect();
    let (cert, _proof) = engine.create_certificate_and_proof_sync(ingots).expect("certificate creation");
    let t: Triple = cert.total_triple;

    // Bound invariants
    assert!(t.tokentorq_remainder < 1000, "tokentorq remainder bound");
    assert!(t.jouletorq_remainder < 3600, "jouletorq remainder bound");

    // Non-negative invariants
    assert!(t.robotorq >= 0);
    assert!(t.tokentorq_remainder >= 0);
    assert!(t.jouletorq_remainder >= 0);

    // Consistency reconstruction
    let reconstructed = triple_to_jouletorq(t);
    assert_eq!(reconstructed, cert.total_jouletorq);
}
