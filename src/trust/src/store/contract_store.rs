use crate::models::contract::Contract;
use anyhow::Result;
use std::fs;
use std::sync::RwLock;
use crate::metrics::METRICS;

pub struct ContractStore {
    inner: RwLock<Vec<Contract>>,
}

impl ContractStore {
    pub fn new() -> Self {
        ContractStore { inner: RwLock::new(Vec::new()) }
    }

    pub fn load_genesis(&self, path: &str) -> Result<()> {
        let data = fs::read_to_string(path)?;
        let c: Contract = serde_json::from_str(&data)?;
        c.validate().map_err(|e| anyhow::anyhow!(e))?;
        let mut w = self.inner.write().unwrap();
        w.push(c);
        // Update served contracts gauge
        METRICS.served_contracts.set(w.len() as i64);
        Ok(())
    }

    pub fn get(&self, contract_id: &str) -> Option<Contract> {
        let r = self.inner.read().unwrap();
        for c in r.iter() {
            if c.contract_id == contract_id {
                return Some(c.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use crate::metrics::METRICS;

    #[test]
    fn test_new_store() {
        let store = ContractStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_load_genesis_valid() {
        let store = ContractStore::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        let genesis_json = r#"
        {
            "contract_id": "genesis-001",
            "version": 1,
            "title": "Genesis Contract",
            "description": "Initial contract",
            "robot_id": "3d-printer-jon",
            "owner_name": "Jonathan Joseph Clark",
            "robostake_required": false,
            "payload": {"key": "value"},
            "created_at": "2025-11-24T00:00:00Z",
            "signature": null
        }
        "#;
        write!(temp_file, "{}", genesis_json).unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = store.load_genesis(path);
        assert!(result.is_ok());

        let contract = store.get("genesis-001");
        assert!(contract.is_some());
        assert_eq!(contract.unwrap().contract_id, "genesis-001");
        // metrics should reflect served contract count
        let m = METRICS.served_contracts.get();
        assert_eq!(m, 1);
    }

    #[test]
    fn test_load_genesis_invalid_json() {
        let store = ContractStore::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "invalid json").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = store.load_genesis(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_genesis_invalid_contract() {
        let store = ContractStore::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_json = r#"
        {
            "contract_id": "",
            "version": 1,
            "title": "Genesis Contract",
            "robot_id": "3d-printer-jon",
            "robostake_required": false,
            "payload": {},
            "created_at": "2025-11-24T00:00:00Z"
        }
        "#;
        write!(temp_file, "{}", invalid_json).unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = store.load_genesis(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent() {
        let store = ContractStore::new();
        assert!(store.get("nonexistent").is_none());
    }
}
