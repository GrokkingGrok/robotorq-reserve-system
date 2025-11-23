use robotorq_vault::{VaultConfig, ShadowCertVault, ShadowStakeVault, VaultMetrics};
use axum::{Router, routing::get, response::IntoResponse};
use std::net::SocketAddr;
use std::sync::Arc;
use robotorq_vault::nats_client::connect_nats;
use robotorq_vault::events::subjects;
use robotorq_vault::models::RoboTorqBatch;
use anyhow::Result;
use async_nats::Subscriber;
use futures_util::stream::StreamExt;
use tracing::{info, error};
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cfg = VaultConfig::from_env();
    info!(config=?cfg, "vault configuration loaded");
    let nats = match connect_nats(&cfg.nats_url).await {
        Ok(c) => c,
        Err(e) => {
            error!(error=%e, nats_url=%cfg.nats_url, "failed to connect to NATS");
            return Err(e);
        }
    };

    let metrics = VaultMetrics::new();
    let cert_vault = Arc::new(ShadowCertVault::new(nats.clone()).with_metrics(metrics.clone()));
    let stake_vault = Arc::new(ShadowStakeVault::new(nats.clone()).with_metrics(metrics.clone()));

    // Spawn subscription processing in its own task so HTTP server can start immediately.
    let cert_vault_sub = cert_vault.clone();
    let stake_vault_sub = stake_vault.clone();
    let nats_sub = nats.clone();
    let sub_task = task::spawn(async move {
        let mut sub: Subscriber = nats_sub.subscribe(subjects::PHASE3_COMPLETED).await?;
        info!(subject = subjects::PHASE3_COMPLETED, "vault subscribed");
        while let Some(msg) = sub.next().await {
            match serde_json::from_slice::<RoboTorqBatch>(&msg.payload) {
                Ok(batch) => {
                    info!(batch_id = %batch.batch_id, cert_count = batch.certificates.len(), "processing batch");
                    for cert in batch.certificates.iter() {
                        if let Err(e) = cert_vault_sub.store_certificate(cert.clone()).await {
                            error!(cert_id=%cert.cert_id, error=%e, "failed to store certificate");
                        }
                    }
                    let returned = batch.total_robostake;
                    stake_vault_sub.increment_available(returned);
                    if let Err(e) = stake_vault_sub.publish_robostake_return(batch.batch_id.clone(), returned).await {
                        error!(batch_id=%batch.batch_id, error=%e, "failed publishing robostake.returned");
                    }
                }
                Err(e) => {
                    error!(error=%e, "invalid batch payload");
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

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
            async move { ([("Content-Type", "text/plain; version=0.0.4")], m.encode()).into_response() }
        }));

    let addr: SocketAddr = "0.0.0.0:8088".parse()?;
    tracing::info!(%addr, "starting vault HTTP server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = task::spawn(async move {
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    // Wait for either task to error (server normally runs indefinitely).
    tokio::select! {
        r = server => { r??; }
        r = sub_task => { r??; }
    }
    Ok(())
}
