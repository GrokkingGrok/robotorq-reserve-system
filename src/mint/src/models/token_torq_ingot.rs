use serde::{Deserialize, Serialize};

/// TokenTorqIngot: 3600-unit ingot produced by Refinery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTorqIngot {
    pub ingot_id: String,
    pub contract_id: String,
    pub joules_total: f64,
    pub robostake_total_micro_rt: i64,
    // Additional fields as needed
}