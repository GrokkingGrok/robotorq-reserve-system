use serde::{Serialize, Deserialize};
use super::RoboTorqCertificate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoboTorqBatch {
    pub event_type: String,            // expect "robotorqcert_batch_completed"
    pub batch_id: String,
    pub created_at: i64,
    pub cert_count: usize,
    pub canonical_total_jouletorq: i64,
    pub certificates: Vec<RoboTorqCertificate>,
}
