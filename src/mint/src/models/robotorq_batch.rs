use serde::{Deserialize, Serialize};
use super::robotorq_certificate::RoboTorqCertificate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqBatch {
    pub batch_id: String,
    /// Creation timestamp (unix nanos)
    pub created_at_nanos: i64,
    pub certificates: Vec<RoboTorqCertificate>,
}