use dashmap::DashMap;
use crate::models::RoboTorqCertificate;
use crate::events::subjects;
use crate::metrics::VaultMetrics;
use async_nats::Client;
use anyhow::Result;
use tracing::info;
use std::sync::Arc;

#[derive(Clone)]
pub struct ShadowCertVault {
    certificates: DashMap<String, RoboTorqCertificate>,
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>, // optional metrics injection
}

impl ShadowCertVault {
    pub fn new(nats: Client) -> Self { Self { certificates: DashMap::new(), nats, metrics: None } }

    pub fn with_metrics(mut self, metrics: Arc<VaultMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub async fn store_certificate(&self, cert: RoboTorqCertificate) -> Result<()> {
        let id = cert.cert_id.clone();
        self.certificates.insert(id.clone(), cert);
        // Publish minimal stored event (batch summary omitted here)
        let evt = serde_json::json!({
            "event_type": "cert_stored",
            "cert_id": id,
        });
        self.nats.publish(subjects::CERT_STORED, serde_json::to_vec(&evt)?.into()).await?;
        if let Some(m) = &self.metrics { m.cert_stored_total.inc(); }
        info!(cert_id=%id, "certificate stored");
        Ok(())
    }

    pub fn robotorq_count_for_contract(&self, contract_id: &str) -> i64 {
        self.certificates.iter().filter(|c| c.contract_ids.iter().any(|cid| cid == contract_id)).count() as i64
    }

    pub fn total_robotorq(&self) -> i64 { self.certificates.len() as i64 }
}
