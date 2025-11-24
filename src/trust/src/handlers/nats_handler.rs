use crate::config::TrustConfig;
use crate::store::contract_store::ContractStore;
use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
struct ErrorReply {
    error: String,
}

impl NatsHandler {
    pub fn new(cfg: Arc<TrustConfig>, store: Arc<ContractStore>) -> Self {
        NatsHandler { cfg, store }
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!(nats_url = %self.cfg.nats_url, "NatsHandler connecting to NATS");

        let client = Client::connect(self.cfg.nats_url.clone()).await?;

        let mut sub = client.subscribe("trust.contract.request").await?;

        tracing::info!("NatsHandler subscribed to trust.contract.request");

        while let Some(msg) = sub.next().await {
            let payload = msg.payload;
            let resp = match serde_json::from_slice::<ContractRequest>(&payload) {
                Ok(req) => {
                    match self.store.get(&req.contract_id) {
                        Some(contract) => match serde_json::to_vec(&contract) {
                            Ok(b) => b,
                            Err(e) => serde_json::to_vec(&ErrorReply { error: format!("serialize_error: {}", e) }).unwrap_or_else(|_| b"{\"error\":\"serialize_failed\"}\".to_vec()),
                        },
                        None => serde_json::to_vec(&ErrorReply { error: "not_found".to_string() }).unwrap(),
                    }
                }
                Err(e) => serde_json::to_vec(&ErrorReply { error: format!("bad_request: {}", e) }).unwrap(),
            };

            if let Err(e) = msg.respond(resp.into()).await {
                tracing::error!("failed to respond to contract request: {}", %e);
            }
        }

        Ok(())
    }
}
