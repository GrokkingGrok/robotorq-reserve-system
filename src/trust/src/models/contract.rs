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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_validate_valid_contract() {
        let contract = Contract {
            contract_id: "genesis-001".to_string(),
            version: 1,
            title: "Genesis Contract".to_string(),
            description: Some("Initial contract".to_string()),
            robot_id: "3d-printer-jon".to_string(),
            owner_name: Some("Jonathan Joseph Clark".to_string()),
            owner_wallet_id: None,
            robostake_required: false,
            payload: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
            signature: None,
        };
        assert!(contract.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_contract_id() {
        let contract = Contract {
            contract_id: "".to_string(),
            version: 1,
            title: "Genesis Contract".to_string(),
            description: Some("Initial contract".to_string()),
            robot_id: "3d-printer-jon".to_string(),
            owner_name: Some("Jonathan Joseph Clark".to_string()),
            owner_wallet_id: None,
            robostake_required: false,
            payload: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
            signature: None,
        };
        assert_eq!(contract.validate(), Err("contract_id empty".to_string()));
    }

    #[test]
    fn test_validate_empty_robot_id() {
        let contract = Contract {
            contract_id: "genesis-001".to_string(),
            version: 1,
            title: "Genesis Contract".to_string(),
            description: Some("Initial contract".to_string()),
            robot_id: "".to_string(),
            owner_name: Some("Jonathan Joseph Clark".to_string()),
            owner_wallet_id: None,
            robostake_required: false,
            payload: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
            signature: None,
        };
        assert_eq!(contract.validate(), Err("robot_id empty".to_string()));
    }
}
