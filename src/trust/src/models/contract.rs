use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_id: String,
    pub version: u32,
    pub title: String,
    pub description: Option<String>,
    pub robot_id: String,
    pub owner_name: Option<String>,
    pub owner_wallet_id: Option<String>,
    pub robostake_required: bool,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub signature: Option<String>,
}

impl Contract {
    pub fn validate(&self) -> Result<(), String> {
        if self.contract_id.trim().is_empty() {
            return Err("contract_id empty".to_string());
        }
        if self.robot_id.trim().is_empty() {
            return Err("robot_id empty".to_string());
        }
        Ok(())
    }
}
