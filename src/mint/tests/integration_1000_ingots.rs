//! Integration-style test: simulate sending 1000 ingots through ProofEngine
//! without wiring external transport. Ensures certificate/proof creation
//! behaves correctly at batch threshold with economic + triple invariants.

use std::time::SystemTime;
use std::sync::Arc;
use mint::engine::proof_engine::ProofEngine;
use mint::config::MintConfig;
use mint::metrics::MintMetrics;
use mint::models::token_torq_ingot::TokenTorqIngot;
use common::triples::{triple_to_jouletorq, jouletorq_to_triple, JOULETORQ_PER_ROBOTORQ};

fn build_ingot(i: usize) -> TokenTorqIngot {
    TokenTorqIngot {
        ingot_id: format!("ingot-{:04}", i),
        contract_id: if i % 2 == 0 { "contract-A".into() } else { "contract-B".into() },
        joules_total: 3600,                // integer joule units
        robostake_total_jouletorq: 3600,   // simplified stake mirror
        unit_count: 3600,                  // expected ore per ingot
        merkle_root: "a".repeat(64),      // placeholder hash
        unit_hashes: vec![],               // omitted for memory efficiency (not validated here)
        timestamp: SystemTime::now(),
        signature: vec![],
    }
}

#[test]
fn create_certificate_from_1000_ingots() {
    // Config with crypto disabled for deterministic speed
    let cfg = Arc::new(MintConfig {
        enable_crypto: false,
        signature_algorithm: "falcon1024".into(),
        ..MintConfig::default()
    });
    let metrics = Arc::new(MintMetrics::new());
    let engine = ProofEngine::new(Arc::clone(&cfg), Arc::clone(&metrics));

    // Build 1000 ingots (alternating contracts for multi-contract aggregation)
    let ingots: Vec<_> = (0..1000).map(build_ingot).collect();
    let total_expected: i64 = ingots.iter().map(|g| g.joules_total).sum();

    let (cert, proof) = engine.create_certificate_and_proof_sync(ingots).expect("certificate and proof");

    // Basic structural assertions
    assert!(cert.cert_id.starts_with("cert-"));
    assert_eq!(cert.total_jouletorq, total_expected);
    assert!(!cert.hash.is_empty(), "certificate hash should be computed");
    assert!(!proof.proof_hash.is_empty(), "proof hash should be computed");
    assert_eq!(proof.certificate_id, cert.cert_id);

    // Economic invariants
    let triple_reconstructed = triple_to_jouletorq(cert.total_triple);
    assert_eq!(triple_reconstructed, cert.total_jouletorq, "triple decomposition must reconstruct total_jouletorq");
    assert!(cert.total_triple.robotorq <= cert.total_jouletorq / JOULETORQ_PER_ROBOTORQ);

    // Contract aggregation: expect both A and B
    assert!(cert.contract_ids.contains(&"contract-A".into()));
    assert!(cert.contract_ids.contains(&"contract-B".into()));

    // High-order triple integrity (re-derive and compare components)
    let expected_triple = jouletorq_to_triple(cert.total_jouletorq);
    assert_eq!(expected_triple.robotorq, cert.total_triple.robotorq);
    assert_eq!(expected_triple.tokentorq_remainder, cert.total_triple.tokentorq_remainder);
    assert_eq!(expected_triple.jouletorq_remainder, cert.total_triple.jouletorq_remainder);

    // Hash determinism sanity: recompute inline payload used in ProofEngine (excluding remainder)
    let recomputed_payload = format!(
        "{}{}{}{}{}{}{}",
        cert.cert_id,
        cert.merkle_root,
        cert.contract_ids.len(),
        cert.timestamp_nanos,
        cert.total_jouletorq,
        cert.total_triple.robotorq,
        cert.total_triple.tokentorq_remainder
    );
    let recomputed_hash = {
        use sha2::{Sha256, Digest};
        let mut h = Sha256::new();
        h.update(recomputed_payload.as_bytes());
        format!("{:x}", h.finalize())
    };
    assert_eq!(recomputed_hash, cert.hash, "certificate hash should match deterministic payload");

    // Proof linkage
    assert_eq!(proof.total_jouletorq, cert.total_jouletorq);
    assert_eq!(proof.total_triple.robotorq, cert.total_triple.robotorq);
}
