use crate::models::{robotorq_batch::RoboTorqBatch, robotorq_certificate::{RoboTorqCertificate, CertStatus}};
use serde_json::Value;
use common::triples::jouletorq_to_triple;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_cert(id: &str, stake: i64, joule: i64) -> RoboTorqCertificate {
    let mut c = RoboTorqCertificate {
        cert_id: id.to_string(),
        robotorq_proof_id: format!("proof-{}", id),
        merkle_root: "b".repeat(64),
        contract_ids: vec!["contract-x".to_string()],
        timestamp_nanos: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64,
        hash: "".into(),
        status: CertStatus::Digital,
        bearer_bond_id: None,
        total_jouletorq: joule,
        total_stake_jouletorq: stake,
        total_triple: jouletorq_to_triple(joule),
    };
    // Hash omitted; not needed for serialization presence test.
    c
}

#[test]
fn batch_json_includes_stake_and_joule_fields() {
    let certs = vec![
        sample_cert("cert-a", 12345, 3600 * 1000),
        sample_cert("cert-b", 23456, 3600 * 2000),
    ];
    let stake_sum: i128 = certs.iter().map(|c| c.total_stake_jouletorq as i128).sum();
    let joule_sum: i128 = certs.iter().map(|c| c.total_jouletorq as i128).sum();
    let batch = RoboTorqBatch {
        event_type: "robotorqcert_batch_completed".into(),
        batch_id: "batch-test".into(),
        created_at_nanos: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64,
        cert_count: certs.len(),
        total_robostake: stake_sum as i64,
        canonical_total_jouletorq: joule_sum as i64,
        certificates: certs,
    };

    batch.validate().expect("batch invariants hold");

    let json = serde_json::to_string(&batch).expect("serialize batch");
    let v: Value = serde_json::from_str(&json).unwrap();

    assert!(v.get("total_robostake").is_some(), "missing total_robostake in batch JSON");
    assert!(v.get("canonical_total_jouletorq").is_some(), "missing canonical_total_jouletorq in batch JSON");
    assert!(v.get("certificates").is_some(), "missing certificates array");
    let certs_json = v.get("certificates").unwrap().as_array().unwrap();
    assert!(certs_json[0].get("total_stake_jouletorq").is_some(), "certificate stake field absent");
    assert!(certs_json[0].get("total_jouletorq").is_some(), "certificate joule field absent");
}
