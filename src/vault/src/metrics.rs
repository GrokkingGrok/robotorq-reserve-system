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
    pub distostream_authorized_total: IntCounter,
    pub distostream_distribution_ticks_total: IntCounter,
    pub contracts_received_total: IntCounter,
    pub contracts_approved_total: IntCounter,
    pub contracts_completed_total: IntCounter,
    pub short_vault_balance_total: IntGauge,
    pub demurrage_applied_total: IntCounter,
    pub wallet_transfers_total: IntCounter,
    pub active_drips_total: IntGauge,
    pub drips_completed_total: IntCounter,
    pub drips_failed_total: IntCounter,
}

impl VaultMetrics {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();
        let cert_stored_total = IntCounter::new("vault_cert_stored_total", "Total certificates stored") .unwrap();
        let robostake_returned_total = IntCounter::new("vault_robostake_returned_total", "Total robostake units returned") .unwrap();
        let stake_allocated_total = IntCounter::new("vault_stake_allocated_total", "Total robostake units allocated") .unwrap();
        let available_robostake = IntGauge::new("vault_available_robostake", "Current available robostake units") .unwrap();
        let deployed_robostake = IntGauge::new("vault_deployed_robostake", "Current deployed robostake units") .unwrap();
        let distostream_authorized_total = IntCounter::new("vault_distostream_authorized_total", "Total distostream authorization events emitted") .unwrap();
        let distostream_distribution_ticks_total = IntCounter::new("vault_distostream_distribution_ticks_total", "Total distostream distribution tick events emitted") .unwrap();
        let contracts_received_total = IntCounter::new("vault_contracts_received_total", "Total contracts received for approval") .unwrap();
        let contracts_approved_total = IntCounter::new("vault_contracts_approved_total", "Total contracts approved and funded") .unwrap();
        let contracts_completed_total = IntCounter::new("vault_contracts_completed_total", "Total contracts marked as completed") .unwrap();
        let short_vault_balance_total = IntGauge::new("vault_short_vault_balance_total", "Total balance across all ShortVaults") .unwrap();
        let demurrage_applied_total = IntCounter::new("vault_demurrage_applied_total", "Total demurrage applied to ShortVaults") .unwrap();
        let wallet_transfers_total = IntCounter::new("vault_wallet_transfers_total", "Total transfers from ShortVaults to wallets") .unwrap();
        let active_drips_total = IntGauge::new("vault_active_drips_total", "Total active drip schedules across all ShortVaults") .unwrap();
        let drips_completed_total = IntCounter::new("vault_drips_completed_total", "Total drip schedules completed") .unwrap();
        let drips_failed_total = IntCounter::new("vault_drips_failed_total", "Total drip schedules that failed") .unwrap();

        registry.register(Box::new(cert_stored_total.clone())).unwrap();
        registry.register(Box::new(robostake_returned_total.clone())).unwrap();
        registry.register(Box::new(stake_allocated_total.clone())).unwrap();
        registry.register(Box::new(available_robostake.clone())).unwrap();
        registry.register(Box::new(deployed_robostake.clone())).unwrap();
        registry.register(Box::new(distostream_authorized_total.clone())).unwrap();
        registry.register(Box::new(distostream_distribution_ticks_total.clone())).unwrap();
        registry.register(Box::new(contracts_received_total.clone())).unwrap();
        registry.register(Box::new(contracts_approved_total.clone())).unwrap();
        registry.register(Box::new(contracts_completed_total.clone())).unwrap();
        registry.register(Box::new(short_vault_balance_total.clone())).unwrap();
        registry.register(Box::new(demurrage_applied_total.clone())).unwrap();
        registry.register(Box::new(wallet_transfers_total.clone())).unwrap();
        registry.register(Box::new(active_drips_total.clone())).unwrap();
        registry.register(Box::new(drips_completed_total.clone())).unwrap();
        registry.register(Box::new(drips_failed_total.clone())).unwrap();

        Arc::new(Self { registry, cert_stored_total, robostake_returned_total, stake_allocated_total, available_robostake, deployed_robostake, distostream_authorized_total, distostream_distribution_ticks_total, contracts_received_total, contracts_approved_total, contracts_completed_total, short_vault_balance_total, demurrage_applied_total, wallet_transfers_total, active_drips_total, drips_completed_total, drips_failed_total })
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
    pub fn inc_distostream_authorized(&self) { self.distostream_authorized_total.inc(); }
    pub fn inc_distostream_distribution_tick(&self) { self.distostream_distribution_ticks_total.inc(); }
    pub fn inc_contracts_received(&self) { self.contracts_received_total.inc(); }
    pub fn inc_contracts_approved(&self) { self.contracts_approved_total.inc(); }
    pub fn inc_contracts_completed(&self) { self.contracts_completed_total.inc(); }
    pub fn set_short_vault_balance(&self, val: i64) { self.short_vault_balance_total.set(val); }
    pub fn inc_demurrage_applied(&self, val: i64) { self.demurrage_applied_total.inc_by(val as u64); }
    pub fn inc_wallet_transfers(&self, val: i64) { self.wallet_transfers_total.inc_by(val as u64); }
    pub fn set_active_drips(&self, val: i64) { self.active_drips_total.set(val); }
    pub fn inc_drips_completed(&self) { self.drips_completed_total.inc(); }
    pub fn inc_drips_failed(&self) { self.drips_failed_total.inc(); }
}
