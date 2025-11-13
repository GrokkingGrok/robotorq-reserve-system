// This file is the **JOB OFFICE** — it keeps track of all work contracts

use crate::types::Contract;  // Import the job agreement blueprint
use std::collections::HashMap;  // Like a filing cabinet: job ID → contract
use lazy_static::lazy_static;   // Magic to create one global office
use std::sync::{Arc, Mutex};    // Tools to safely share the office

// ────────────────────────────────────────────────────────────────
// THE JOB OFFICE (ContractManager)
// ────────────────────────────────────────────────────────────────

/// This is like the **HR department** that manages every job contract
pub struct ContractManager {
    /// A filing cabinet: job number → full job details
    /// Example: "contract-001" → all the rules for that job
    contracts: HashMap<String, Contract>,
}

impl ContractManager {
    /// Create **one global job office** that everyone uses
    /// (Like a school office — only one!)
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<ContractManager>> =
                Arc::new(Mutex::new(ContractManager { contracts: HashMap::new() }));
        }
        INSTANCE.clone()
    }

    /// Look up a job by its number
    /// Returns the full job details, or nothing if it doesn't exist
    pub fn get_contract(&self, id: &str) -> Option<Contract> {
        self.contracts.get(id).cloned()  // Make a copy and return it
    }

    /// Get a list of **all job numbers**
    /// Like reading the labels on the filing cabinet
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()  // Return all job IDs
    }

    /// Add a **new job** to the filing cabinet
    /// Like putting a new folder in the drawer
    pub fn add_contract(&mut self, contract: Contract) {
        self.contracts.insert(contract.id.clone(), contract);
    }

    /// **Approve a job** so the robot can start working
    /// Like stamping "APPROVED" on the contract
    pub fn authorize_contract(&mut self, id: &str) {
        if let Some(c) = self.contracts.get_mut(id) {
            c.authorized = true;  // Flip the switch to "yes"
        }
    }
}