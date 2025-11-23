use prometheus::{IntCounter, IntGauge, Registry, Encoder, TextEncoder};
use std::sync::Arc;

#[derive(Clone)]
pub struct VaultMetrics {
    pub registry: Registry,
    pub cert_stored_total: IntCounter,
    pub robostake_returned_total: IntCounter,
    pub stake_allocated_total: IntCounter,
    pub available_robostake: IntGauge,
    pub deployed_robostake: IntGauge,
}

impl VaultMetrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();
        let cert_stored_total = IntCounter::new("vault_cert_stored_total", "Total certificates stored") .unwrap();
        let robostake_returned_total = IntCounter::new("vault_robostake_returned_total", "Total robostake units returned") .unwrap();
        let stake_allocated_total = IntCounter::new("vault_stake_allocated_total", "Total robostake units allocated") .unwrap();
        let available_robostake = IntGauge::new("vault_available_robostake", "Current available robostake units") .unwrap();
        let deployed_robostake = IntGauge::new("vault_deployed_robostake", "Current deployed robostake units") .unwrap();

        registry.register(Box::new(cert_stored_total.clone())).unwrap();
        registry.register(Box::new(robostake_returned_total.clone())).unwrap();
        registry.register(Box::new(stake_allocated_total.clone())).unwrap();
        registry.register(Box::new(available_robostake.clone())).unwrap();
        registry.register(Box::new(deployed_robostake.clone())).unwrap();

        Arc::new(Self { registry, cert_stored_total, robostake_returned_total, stake_allocated_total, available_robostake, deployed_robostake })
    }

    pub fn encode(&self) -> String {
        let mf = self.registry.gather();
        let mut buf = Vec::new();
        TextEncoder::new().encode(&mf, &mut buf).unwrap();
        String::from_utf8(buf).unwrap_or_default()
    }

    // convenience for updating gauges externally if needed
    pub fn set_available(&self, val: i64) { self.available_robostake.set(val as i64); }
    pub fn set_deployed(&self, val: i64) { self.deployed_robostake.set(val as i64); }
}
