use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct JouleTorqOre {
    pub digger_id: String,
    pub contract_id: String,
    pub tokens_generated: u64,
    pub joules: u64,
    pub milestone_index: u32,
    pub timestamp: u64,
    pub proof_of_work: Option<String>, // base64 photo
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    pub authorized: bool,
    pub torq: u16,
    pub max_token_throughput: u64,
    pub interval_seconds: u64,
    pub total_tokens: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DiggerConfig {
    pub id: String,
    pub power_kw: f64,
    pub max_token_throughput: u64,
}
