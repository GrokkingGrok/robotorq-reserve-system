use std::sync::atomic::{AtomicI64, Ordering};
use async_nats::Client;
use anyhow::{Result, anyhow};
use crate::events::subjects;
use tracing::info;

/// ShadowStakeVault (MVP simplified)
///
/// CURRENT SCOPE:
///   - Tracks only whole RoboStake units (integral RoboTorq) in `available` and `deployed`.
///   - Ignores TokenTorq and JouleTorq remainders; events publish zero placeholders.
///   - Accepts returned stake (`total_robostake`) from Phase3 batches independently of certificate count.
/// FUTURE (commentary / design intent):
///   - Will reintroduce hierarchical triple (R, T remainder, J remainder) for partial progress accounting,
///     fractional distribution streams, and precise reserve normalization.
///   - Remainders enable expressing partially accumulated ingots or ore toward the next certificate without
///     prematurely incrementing whole RoboTorq reserve.
///   - Allocation and return flows will use a single normalization pass to carry J -> T -> R.
/// WHY KEEP IT SIMPLE NOW:
///   - MVP only receives whole stake returns; complexity of partials adds cognitive overhead with no runtime benefit.
///   - Isolation clarifies semantics: `available_robostake` is the only mutable economic number in StakeVault.
pub struct ShadowStakeVault {
    available_robostake: AtomicI64,
    deployed_robostake: AtomicI64,
    nats: Client,
}

impl ShadowStakeVault {
    pub fn new(nats: Client) -> Self {
        Self {
            available_robostake: AtomicI64::new(0),
            deployed_robostake: AtomicI64::new(0),
            nats,
        }
    }

    /// Increment available stake by whole RoboStake units.
    /// FUTURE: accept (R,T,J) and normalize; for now only whole units are meaningful.
    pub fn increment_available(&self, robostake_units: i64) {
        self.available_robostake.fetch_add(robostake_units, Ordering::SeqCst);
    }

    /// Whole available RoboStake.
    pub fn available_robostake(&self) -> i64 {
        self.available_robostake.load(Ordering::SeqCst)
    }

    /// Whole deployed RoboStake.
    pub fn deployed_robostake(&self) -> i64 {
        self.deployed_robostake.load(Ordering::SeqCst)
    }

    /// Allocate whole RoboStake units to a contract.
    /// FUTURE: extend to (R,T,J) partial allocations when remainders become meaningful.
    pub async fn allocate(&self, contract_id: &str, robostake_units: i64) -> Result<()> {
        if robostake_units < 0 { return Err(anyhow!("negative allocation not allowed")); }
        let avail = self.available_robostake();
        if robostake_units > avail { return Err(anyhow!("insufficient reserve")); }
        self.available_robostake.fetch_sub(robostake_units, Ordering::SeqCst);
        self.deployed_robostake.fetch_add(robostake_units, Ordering::SeqCst);
        let evt = serde_json::json!({
            "event_type": "stake_allocated",
            "contract_id": contract_id,
            "robostake": robostake_units,
            "tokentorq_remainder": 0, // placeholder for future triple expansion
            "jouletorq_remainder": 0,
        });
        self.nats.publish(subjects::STAKE_ALLOCATED, serde_json::to_vec(&evt)?.into()).await?;
        info!(contract_id=%contract_id, robostake_units=robostake_units, "allocation applied");
        Ok(())
    }

    /// Publish stake return event. Only whole RoboStake for MVP; remainder fields are placeholders.
    pub async fn publish_robostake_return(&self, batch_id: String, robostake_units: i64) -> Result<()> {
        let evt = serde_json::json!({
            "event_type": "robostake_returned",
            "batch_id": batch_id,
            "robostake": robostake_units,
            "tokentorq_remainder": 0,
            "jouletorq_remainder": 0,
        });
        self.nats.publish(subjects::ROBOSTAKE_RETURNED, serde_json::to_vec(&evt)?.into()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_nats::connect;

    // This test expects a local NATS; skip if unavailable
    #[tokio::test]
    async fn allocation_works_in_memory_whole_units() {
        let nats = connect("nats://127.0.0.1:4222").await.expect("nats required for test");
        let sv = ShadowStakeVault::new(nats);
        sv.increment_available(5);
        assert_eq!(sv.available_robostake(), 5);
        sv.allocate("contract-1", 2).await.unwrap();
        assert_eq!(sv.available_robostake(), 3);
        assert_eq!(sv.deployed_robostake(), 2);
    }
}
