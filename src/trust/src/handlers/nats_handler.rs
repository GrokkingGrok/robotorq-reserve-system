use crate::config::TrustConfig;
use crate::store::contract_store::ContractStore;
use anyhow::Result;
use std::sync::Arc;

pub struct NatsHandler {
    cfg: Arc<TrustConfig>,
    store: Arc<ContractStore>,
}

impl NatsHandler {
    pub fn new(cfg: Arc<TrustConfig>, store: Arc<ContractStore>) -> Self {
        NatsHandler { cfg, store }
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("NatsHandler starting (stub)");
        // TODO: wire async-nats subscription and handle request/reply for `trust.contract.request`
        Ok(())
    }
}
