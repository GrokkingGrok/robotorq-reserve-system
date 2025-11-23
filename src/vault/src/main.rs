use robotorq_vault::{VaultConfig, ShadowCertVault, ShadowStakeVault, VaultMetrics};
use axum::{Router, routing::get, http::StatusCode, Json};
use axum::extract::State;
use std::net::SocketAddr;
use std::sync::Arc;
use robotorq_vault::nats_client::connect_nats;
use robotorq_vault::events::subjects;
use robotorq_vault::models::RoboTorqBatch;
use anyhow::Result;
use futures_util::stream::StreamExt;
use tracing::{info, error};
use tokio::task;
use serde_json::json;

#[derive(Clone)]
struct AppState {
    cert_vault: Arc<ShadowCertVault>,
    stake_vault: Arc<ShadowStakeVault>,
    metrics: Arc<VaultMetrics>,
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "certificates": state.cert_vault.total_robotorq(),
        "available_robostake": state.stake_vault.available_robostake(),
        "deployed_robostake": state.stake_vault.deployed_robostake(),
    }))
}

async fn metrics(State(state): State<AppState>) -> (StatusCode, [(String, String); 1], String) {
    let body = state.metrics.encode();
    (
        StatusCode::OK,
        [("Content-Type".to_string(), "text/plain; version=0.0.4".to_string())],
        body,
    )
}

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

    let vault_metrics = VaultMetrics::new();
    let cert_vault = Arc::new(ShadowCertVault::new(nats.clone()).with_metrics(vault_metrics.clone()));
    let stake_vault = Arc::new(ShadowStakeVault::new(nats.clone()).with_metrics(vault_metrics.clone()));
    let state = AppState { cert_vault: cert_vault.clone(), stake_vault: stake_vault.clone(), metrics: vault_metrics.clone() };

    // Subscription processing task (detached).
    let sub_state = state.clone();
    task::spawn(async move {
        match nats.subscribe(subjects::PHASE3_COMPLETED).await {
            Ok(mut sub) => {
                info!(subject = subjects::PHASE3_COMPLETED, "vault subscribed");
                while let Some(msg) = sub.next().await {
                    match serde_json::from_slice::<RoboTorqBatch>(&msg.payload) {
                        Ok(batch) => {
                            info!(batch_id = %batch.batch_id, cert_count = batch.certificates.len(), "processing batch");
                            for cert in batch.certificates.iter() {
                                if let Err(e) = sub_state.cert_vault.store_certificate(cert.clone()).await {
                                    error!(cert_id=%cert.cert_id, error=%e, "failed to store certificate");
                                }
                            }
                            let returned = batch.total_robostake;
                            sub_state.stake_vault.increment_available(returned);
                            if let Err(e) = sub_state.stake_vault.publish_robostake_return(batch.batch_id.clone(), returned).await {
                                error!(batch_id=%batch.batch_id, error=%e, "failed publishing robostake.returned");
                            }
                        }
                        Err(e) => {
                            error!(error=%e, "invalid batch payload");
                        }
                    }
                }
            }
            Err(e) => error!(error=%e, "failed to subscribe to phase3 completed subject"),
        }
    });

    // HTTP server for /health and /metrics (async handlers)
    let health_cert_vault = cert_vault.clone();
    let health_stake_vault = stake_vault.clone();
    let metrics_arc = metrics.clone();

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8088".parse()?;
    tracing::info!(%addr, "starting vault HTTP server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = task::spawn(async move {
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    server.await??;
    Ok(())
}
