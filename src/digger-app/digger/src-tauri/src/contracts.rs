use crate::types::Contract;
use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

pub struct ContractManager {
    contracts: HashMap<String, Contract>,
}

impl ContractManager {
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<ContractManager>> =
                Arc::new(Mutex::new(ContractManager { contracts: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    pub fn get_contract(&self, id: &str) -> Option<Contract> {
        self.contracts.get(id).cloned()
    }

    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    pub fn add_contract(&mut self, contract: Contract) {
        self.contracts.insert(contract.id.clone(), contract);
    }

    pub fn authorize_contract(&mut self, id: &str) {
        if let Some(c) = self.contracts.get_mut(id) {
            c.authorized = true;
        }
    }
}
