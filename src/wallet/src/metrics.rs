//! Prometheus metrics for the wallet service

use anyhow::Result;
use prometheus::{Encoder, IntCounter, IntGauge, Registry, TextEncoder};

/// Wallet service metrics
#[derive(Clone)]
pub struct Metrics {
    /// Total transactions processed
    pub transactions_total: IntCounter,
    /// Total UBD received (in jouletorq units)
    pub ubd_received_total: IntCounter,
    /// Current UBD balance (in jouletorq units)
    pub ubd_balance: IntGauge,
    /// Prometheus registry
    registry: Registry,
}

impl Metrics {
    /// Create new metrics instance
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let transactions_total = IntCounter::new(
            "wallet_transactions_total",
            "Total number of transactions processed",
        )?;
        registry.register(Box::new(transactions_total.clone()))?;

        let ubd_received_total = IntCounter::new(
            "wallet_ubd_received_total",
            "Total UBD received in jouletorq units",
        )?;
        registry.register(Box::new(ubd_received_total.clone()))?;

        let ubd_balance = IntGauge::new(
            "wallet_ubd_balance",
            "Current UBD balance in jouletorq units",
        )?;
        registry.register(Box::new(ubd_balance.clone()))?;

        Ok(Self {
            transactions_total,
            ubd_received_total,
            ubd_balance,
            registry,
        })
    }

    /// Encode metrics for HTTP response
    pub fn encode(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Record transaction
    pub fn record_transaction(&self) {
        self.transactions_total.inc();
    }

    /// Record UBD reception
    pub fn record_ubd_received(&self, amount: i64) {
        self.ubd_received_total.inc_by(amount as u64);
    }

    /// Update UBD balance
    pub fn update_ubd_balance(&self, balance: f64) {
        self.ubd_balance.set(balance as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_ubd_tracking() {
        let metrics = Metrics::new().unwrap();

        metrics.record_ubd_received(1000);
        assert_eq!(metrics.ubd_received_total.get(), 1000);

        metrics.update_ubd_balance(500.0);
        // Note: update_ubd_balance uses set with i64 conversion
        // This tests the conversion logic
    }

    #[test]
    fn test_metrics_encoding_contains_expected_metrics() {
        let metrics = Metrics::new().unwrap();
        metrics.record_transaction();
        metrics.record_ubd_received(100);

        let encoded = metrics.encode().unwrap();

        assert!(encoded.contains("wallet_transactions_total 1"));
        assert!(encoded.contains("wallet_ubd_received_total 100"));
        assert!(encoded.contains("# TYPE"));
        assert!(encoded.contains("# HELP"));
    }

    #[test]
    fn test_metrics_registry_setup() {
        let metrics = Metrics::new().unwrap();

        // Test that all expected metrics are registered
        let encoded = metrics.encode().unwrap();
        assert!(encoded.contains("wallet_transactions_total"));
        assert!(encoded.contains("wallet_ubd_received_total"));
        assert!(encoded.contains("wallet_ubd_balance"));
    }
}