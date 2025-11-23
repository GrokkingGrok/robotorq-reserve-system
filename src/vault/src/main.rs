use robotorq_vault::{VaultConfig, ShadowCertVault, ShadowStakeVault, VaultMetrics};
use axum::{Router, routing::get};
use std::net::SocketAddr;
use std::sync::Arc;
use robotorq_vault::nats_client::connect_nats;
use robotorq_vault::events::subjects;
use robotorq_vault::models::RoboTorqBatch;
use anyhow::Result;
use async_nats::Subscriber;
use futures_util::stream::StreamExt;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cfg = VaultConfig::from_env();
    let nats = connect_nats(&cfg.nats_url).await?;

    let metrics = VaultMetrics::new();
    let cert_vault = Arc::new(ShadowCertVault::new(nats.clone()).with_metrics(metrics.clone()));
    let stake_vault = Arc::new(ShadowStakeVault::new(nats.clone()).with_metrics(metrics.clone()));

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
                // Update stake vault reserve using explicit total_robostake from batch (NOT cert_count derived)
                let returned = batch.total_robostake;
                stake_vault.increment_available(returned);
                if let Err(e) = stake_vault.publish_robostake_return(batch.batch_id.clone(), returned).await {
                    error!(batch_id=%batch.batch_id, error=%e, "failed publishing robostake.returned");
                }
            }
            Err(e) => {
                error!(error=%e, "invalid batch payload");
            }
        }
    }

    // HTTP server for /health and /metrics (async handlers)
    let health_cert_vault = cert_vault.clone();
    let health_stake_vault = stake_vault.clone();
    let metrics_arc = metrics.clone();

    let app = Router::new()
        .route("/health", get(move || {
            let cv = health_cert_vault.clone();
            let sv = health_stake_vault.clone();
            async move {
                let body = serde_json::json!({
                    "certificates": cv.total_robotorq(),
                    "available_robostake": sv.available_robostake(),
                    "deployed_robostake": sv.deployed_robostake(),
                });
                axum::Json(body)
            }
        }))
        .route("/metrics", get(move || {
            let m = metrics_arc.clone();
            async move { m.encode() }
        }));

    let addr: SocketAddr = "0.0.0.0:8088".parse()?;
    tracing::info!(%addr, "starting vault HTTP server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
