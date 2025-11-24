use crate::models::contract::Contract;
use anyhow::Result;
use std::fs;
use std::sync::RwLock;

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
