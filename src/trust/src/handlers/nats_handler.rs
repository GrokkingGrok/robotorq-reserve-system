use crate::config::TrustConfig;
use crate::metrics::METRICS;
use crate::store::contract_store::ContractStore;
use anyhow::Result;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct NatsHandler {
    cfg: Arc<TrustConfig>,
    store: Arc<ContractStore>,
}

#[derive(Debug, Deserialize)]
struct ContractRequest {
    contract_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorReply {
    error: ErrorBody,
}


impl NatsHandler {
    pub fn new(cfg: Arc<TrustConfig>, store: Arc<ContractStore>) -> Self {
        NatsHandler { cfg, store }
    }

    pub async fn start(&self, mut shutdown: tokio::sync::watch::Receiver<bool>) -> Result<()> {
        tracing::info!(nats_url = %self.cfg.nats_url, "NatsHandler connecting to NATS");

        let client = async_nats::connect(self.cfg.nats_url.clone()).await?;

        let mut sub = client.subscribe("trust.contract.request").await?;

        tracing::info!("NatsHandler subscribed to trust.contract.request");

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    tracing::info!("NatsHandler received shutdown signal");
                    break;
                }
                maybe = sub.next() => {
                    if maybe.is_none() {
                        tracing::warn!("NATS subscription closed");
                        break;
                    }
                    let msg = maybe.unwrap();
                    let payload = msg.payload;
                    if payload.len() > self.cfg.max_request_bytes {
                        METRICS.requests_bad.inc();
                        let reply = ErrorReply { error: ErrorBody { code: "payload_too_large".to_string(), message: format!("request exceeds {} bytes", self.cfg.max_request_bytes) }};
                        if let Some(reply_to) = msg.reply.clone() {
                            if let Ok(data) = serde_json::to_vec(&reply) { let _ = client.publish(reply_to, data.into()).await; }
                        }
                        continue;
                    }
                    let start = Instant::now();
                    METRICS.requests_total.inc();
                    let resp = match serde_json::from_slice::<ContractRequest>(&payload) {
                        Ok(req) => {
                            match self.store.get(&req.contract_id) {
                                Some(contract) => match serde_json::to_vec(&contract) {
                                    Ok(b) => b,
                                    Err(e) => serde_json::to_vec(&ErrorReply { error: ErrorBody { code: "serialize_error".to_string(), message: e.to_string() } }).unwrap_or_else(|_| b"{\"error\":{\"code\":\"serialize_failed\",\"message\":\"internal error\"}}".to_vec()),
                                },
                                None => {
                                    METRICS.requests_not_found.inc();
                                    serde_json::to_vec(&ErrorReply { error: ErrorBody { code: "not_found".to_string(), message: "contract not found".to_string() } }).unwrap()
                                },
                            }
                        }
                        Err(e) => {
                            METRICS.requests_bad.inc();
                            serde_json::to_vec(&ErrorReply { error: ErrorBody { code: "bad_request".to_string(), message: e.to_string() } }).unwrap()
                        },
                    };
                    let elapsed = start.elapsed();
                    METRICS.request_latency.observe(elapsed.as_secs_f64());

                    if let Some(reply_to) = msg.reply {
                        if let Err(e) = client.publish(reply_to.clone(), resp.into()).await {
                            METRICS.requests_bad.inc();
                            tracing::error!(reply = %reply_to, error = %e, "failed to publish reply to subscriber");
                        }
                    } else {
                        tracing::warn!("contract request had no reply subject");
                    }
                }
            }
        }

        Ok(())
    }
}
