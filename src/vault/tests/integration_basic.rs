use robotorq_vault::{ShadowCertVault, ShadowStakeVault};
use robotorq_vault::models::{RoboTorqCertificate, RoboTorqBatch};
use async_nats::connect;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn batch_ingestion_updates_stake() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("nats required");
    let cert_vault = ShadowCertVault::new(nats.clone());
    let stake_vault = ShadowStakeVault::new(nats.clone());

    let certs: Vec<RoboTorqCertificate> = (0..3).map(|i| RoboTorqCertificate {
        cert_id: format!("CERT-{i}"),
        merkle_root: "deadbeef".into(),
        tree_height: Some(10),
        contract_ids: vec!["printer-coin-42".into()],
        minted_at: 0,
    }).collect();
    let batch = RoboTorqBatch {
        event_type: "robotorqcert_batch_completed".into(),
        batch_id: "batch-01".into(),
        created_at: 0,
        cert_count: certs.len(),          // 3 certificates
        total_robostake: 5,               // returned stake value differs from cert_count
        canonical_total_jouletorq: (certs.len() as i64) * 3_600_000,
        certificates: certs.clone(),
    };

    // Simulate main loop logic directly
    for c in batch.certificates.iter() { cert_vault.store_certificate(c.clone()).await.unwrap(); }
    stake_vault.increment_available(batch.total_robostake);

    assert_eq!(cert_vault.total_robotorq(), 3, "certificate count unaffected by stake value");
    assert_eq!(stake_vault.available_robostake(), 5, "stake reserve uses total_robostake not cert_count");

    // Allocate 2 RoboTorq from returned stake (5 total -> expect 3 remaining)
    stake_vault.allocate("contract-X", 2).await.unwrap();
    assert_eq!(stake_vault.available_robostake(), 3, "available should drop by allocated amount");
    assert_eq!(stake_vault.deployed_robostake(), 2, "deployed should reflect allocation");

    sleep(Duration::from_millis(10)).await;
}
