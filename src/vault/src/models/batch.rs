use serde::{Serialize, Deserialize};
use super::RoboTorqCertificate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoboTorqBatch {
    pub event_type: String,             // expect "robotorqcert_batch_completed"
    pub batch_id: String,
    pub created_at: i64,
    pub cert_count: usize,              // number of minted whole RoboTorq certificates
    pub total_robostake: i64,           // returned RoboStake (R units) – NOT derived from cert_count; may be >, <, or == cert_count
    pub canonical_total_jouletorq: i64,  // full physics sum for future DistoVault; not applied to StakeVault directly
    pub certificates: Vec<RoboTorqCertificate>,
}
