use robotorq_vault::models::{RoboTorqCertificate, RoboTorqBatch};
use async_nats::connect;
use serde_json;

#[tokio::main]
async fn main() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("nats connect");
    let certs = vec![
        RoboTorqCertificate {
            cert_id: "TEST-CERT-1".into(),
            merkle_root: "deadbeef".into(),
            tree_height: Some(10),
            contract_ids: vec!["test-contract".into()],
            minted_at: 1732224000000000000,
        }
    ];
    let batch = RoboTorqBatch {
        event_type: "robotorqcert_batch_completed".into(),
        batch_id: "test-batch-001".into(),
        created_at: 1732224000000000000,
        cert_count: certs.len(),
        canonical_total_jouletorq: (certs.len() as i64) * 3_600_000,
        certificates: certs,
    };
    let payload = serde_json::to_vec(&batch).unwrap();
    nats.publish("vault.phase3.completed", payload.into()).await.unwrap();
    println!("Published test batch");
}