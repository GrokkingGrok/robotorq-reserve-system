use std::sync::atomic::{AtomicI64, Ordering};
use async_nats::Client;
use anyhow::{Result, anyhow};
use crate::events::subjects;
use crate::models::{Triple, normalize_triple};
use tracing::info;

pub struct ShadowStakeVault {
    available_robotorq: AtomicI64,
    available_tokentorq_remainder: AtomicI64,
    available_jouletorq_remainder: AtomicI64,
    deployed_robotorq: AtomicI64,
    deployed_tokentorq_remainder: AtomicI64,
    deployed_jouletorq_remainder: AtomicI64,
    nats: Client,
}

impl ShadowStakeVault {
    pub fn new(nats: Client) -> Self {
        Self {
            available_robotorq: AtomicI64::new(0),
            available_tokentorq_remainder: AtomicI64::new(0),
            available_jouletorq_remainder: AtomicI64::new(0),
            deployed_robotorq: AtomicI64::new(0),
            deployed_tokentorq_remainder: AtomicI64::new(0),
            deployed_jouletorq_remainder: AtomicI64::new(0),
            nats,
        }
    }

    pub fn increment_available(&self, r: i64, t: i64, j: i64) {
        let Triple { robotorq, tokentorq_remainder, jouletorq_remainder } = normalize_triple(r, t, j);
        self.available_robotorq.fetch_add(robotorq, Ordering::SeqCst);
        self.available_tokentorq_remainder.fetch_add(tokentorq_remainder, Ordering::SeqCst);
        self.available_jouletorq_remainder.fetch_add(jouletorq_remainder, Ordering::SeqCst);
    }

    pub fn available(&self) -> Triple {
        normalize_triple(
            self.available_robotorq.load(Ordering::SeqCst),
            self.available_tokentorq_remainder.load(Ordering::SeqCst),
            self.available_jouletorq_remainder.load(Ordering::SeqCst),
        )
    }

    pub fn deployed(&self) -> Triple {
        normalize_triple(
            self.deployed_robotorq.load(Ordering::SeqCst),
            self.deployed_tokentorq_remainder.load(Ordering::SeqCst),
            self.deployed_jouletorq_remainder.load(Ordering::SeqCst),
        )
    }

    pub async fn allocate(&self, contract_id: &str, r: i64, t: i64, j: i64) -> Result<()> {
        if r < 0 || t < 0 || j < 0 { return Err(anyhow!("negative allocation not allowed")); }
        let avail = self.available();
        if r > avail.robotorq || t > avail.tokentorq_remainder || j > avail.jouletorq_remainder {
            return Err(anyhow!("insufficient reserve"));
        }
        // Apply allocation
        self.available_robotorq.fetch_sub(r, Ordering::SeqCst);
        self.available_tokentorq_remainder.fetch_sub(t, Ordering::SeqCst);
        self.available_jouletorq_remainder.fetch_sub(j, Ordering::SeqCst);
        self.deployed_robotorq.fetch_add(r, Ordering::SeqCst);
        self.deployed_tokentorq_remainder.fetch_add(t, Ordering::SeqCst);
        self.deployed_jouletorq_remainder.fetch_add(j, Ordering::SeqCst);
        let evt = serde_json::json!({
            "event_type": "stake_allocated",
            "contract_id": contract_id,
            "robotorq": r,
            "tokentorq_remainder": t,
            "jouletorq_remainder": j,
        });
        self.nats.publish(subjects::STAKE_ALLOCATED, serde_json::to_vec(&evt)?.into()).await?;
        info!(contract_id=%contract_id, r=r, t=t, j=j, "allocation applied");
        Ok(())
    }

    pub async fn publish_robostake_return(&self, batch_id: String, r: i64, t: i64, j: i64) -> Result<()> {
        let evt = serde_json::json!({
            "event_type": "robostake_returned",
            "batch_id": batch_id,
            "robotorq": r,
            "tokentorq_remainder": t,
            "jouletorq_remainder": j,
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
    async fn allocation_works_in_memory() {
        let nats = connect("nats://127.0.0.1:4222").await.expect("nats required for test");
        let sv = ShadowStakeVault::new(nats);
        sv.increment_available(5, 10, 100);
        assert_eq!(sv.available().robotorq, 5);
        sv.allocate("contract-1", 2, 0, 0).await.unwrap();
        assert_eq!(sv.available().robotorq, 3);
        assert_eq!(sv.deployed().robotorq, 2);
    }
}
