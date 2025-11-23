use robotorq_vault::{VaultConfig, ShadowCertVault, ShadowStakeVault};
use robotorq_vault::nats_client::connect_nats;
use robotorq_vault::events::subjects;
use robotorq_vault::models::RoboTorqBatch;
use anyhow::Result;
use async_nats::Subscriber;
use futures_util::stream::StreamExt;
use tracing::{info, error};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cfg = VaultConfig::from_env();
    let nats = connect_nats(&cfg.nats_url).await?;

    let cert_vault = Arc::new(ShadowCertVault::new(nats.clone()));
    let stake_vault = Arc::new(ShadowStakeVault::new(nats.clone()));

    // Subscribe to Phase3 completion events
    let mut sub: Subscriber = nats.subscribe(subjects::PHASE3_COMPLETED).await?;
    info!(subject = subjects::PHASE3_COMPLETED, "vault subscribed");

    while let Some(msg) = sub.next().await {
        match serde_json::from_slice::<RoboTorqBatch>(&msg.payload) {
            Ok(batch) => {
                info!(batch_id = %batch.batch_id, cert_count = batch.certificates.len(), "processing batch");
                // Store certificates
                for cert in batch.certificates.iter() {
                    if let Err(e) = cert_vault.store_certificate(cert.clone()).await {
                        error!(cert_id=%cert.cert_id, error=%e, "failed to store certificate");
                    }
                }
                // Update stake vault reserve (each cert = 1 RoboTorq)
                let added = batch.certificates.len() as i64;
                stake_vault.increment_available(added, 0, 0);
                if let Err(e) = stake_vault.publish_robostake_return(batch.batch_id.clone(), added, 0, 0).await {
                    error!(batch_id=%batch.batch_id, error=%e, "failed publishing robostake.returned");
                }
            }
            Err(e) => {
                error!(error=%e, "invalid batch payload");
            }
        }
    }

    Ok(())
}
