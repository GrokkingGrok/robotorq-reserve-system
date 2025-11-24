use serde::{Deserialize, Serialize};
use super::robotorq_certificate::RoboTorqCertificate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqBatch {
    pub event_type: String,            // mirror vault expectation: "robotorqcert_batch_completed"
    pub batch_id: String,
    /// Creation timestamp (unix nanos)
    pub created_at_nanos: i64,
    pub cert_count: usize,             // number of certificates in batch
    pub total_robostake: i64,          // aggregated stake from certificates
    pub canonical_total_jouletorq: i64,// aggregated joule-torq from certificates
    pub certificates: Vec<RoboTorqCertificate>,
}

impl RoboTorqBatch {
    /// Basic invariant validation for stake and joule totals.
    pub fn validate(&self) -> Result<(), String> {
        if self.certificates.len() != self.cert_count { return Err("cert_count mismatch".into()); }
        let stake_sum: i128 = self.certificates.iter().map(|c| c.total_stake_jouletorq as i128).sum();
        let joule_sum: i128 = self.certificates.iter().map(|c| c.total_jouletorq as i128).sum();
        if stake_sum < 0 || joule_sum < 0 { return Err("negative economic totals".into()); }
        if stake_sum != self.total_robostake as i128 { return Err("total_robostake mismatch".into()); }
        if joule_sum != self.canonical_total_jouletorq as i128 { return Err("canonical_total_jouletorq mismatch".into()); }
        Ok(())
    }
}