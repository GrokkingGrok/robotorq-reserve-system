use serde::{Serialize, Deserialize};
use crate::{ContractId, PartyId, InvariantError, ConfigError};
use crate::hashing::hash_struct;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractStatus {
	Draft,
	Active,
	Suspended,
	Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartyRole {
	Builder,
	Supplier,
	Distributor,
	Mint,
	Refinery,
	Robot,
	Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
	pub id: PartyId,
	pub role: PartyRole,
	pub name: String,
	pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTerms {
	pub schema_version: u32,
	pub nats_subject_prefix: String, // e.g., "robot.unmapped.v1"
	pub max_batch_tokens: u32,       // upper bound for batch size
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
	pub id: ContractId,
	pub status: ContractStatus,
	pub parties: Vec<Party>,
	pub terms: ContractTerms,
	pub created_at_ms: i128,
	pub hash: [u8; 32],
}

impl Contract {
	pub fn new(parties: Vec<Party>, terms: ContractTerms, created_at_ms: i128) -> Result<Self, InvariantError> {
		// Basic validations
		if parties.is_empty() {
			return Err(InvariantError::from(ConfigError::Invalid("Contract must have at least one party".to_string())));
		}
		for p in &parties {
			if p.name.trim().is_empty() {
				return Err(InvariantError::from(ConfigError::Invalid("Contract party name empty".to_string())));
			}
		}
		if terms.max_batch_tokens == 0 {
			return Err(InvariantError::from(ConfigError::Invalid("max_batch_tokens must be > 0".to_string())));
		}
		if terms.nats_subject_prefix.trim().is_empty() {
			return Err(InvariantError::from(ConfigError::Invalid("nats_subject_prefix cannot be empty".to_string())));
		}

		let provisional = Self {
			id: ContractId::new(),
			status: ContractStatus::Draft,
			parties,
			terms,
			created_at_ms,
			hash: [0u8;32],
		};
		let hash = hash_struct(&provisional);
		Ok(Self { hash, ..provisional })
	}

	pub fn activate(mut self) -> Self {
		self.status = ContractStatus::Active;
		let hash = hash_struct(&self);
		self.hash = hash;
		self
	}

	pub fn suspend(mut self) -> Self {
		self.status = ContractStatus::Suspended;
		let hash = hash_struct(&self);
		self.hash = hash;
		self
	}

	pub fn terminate(mut self) -> Self {
		self.status = ContractStatus::Terminated;
		let hash = hash_struct(&self);
		self.hash = hash;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample_party(name: &str, role: PartyRole) -> Party {
		Party { id: PartyId::new(), role, name: name.to_string(), public_key: "pubkey-abc".to_string() }
	}

	fn sample_terms() -> ContractTerms {
		ContractTerms { schema_version: 1, nats_subject_prefix: "robot.unmapped.v1".to_string(), max_batch_tokens: 10_000 }
	}

	#[test]
	fn contract_new_ok_and_activate() {
		let c = Contract::new(vec![sample_party("issuer", PartyRole::Builder), sample_party("counter", PartyRole::Supplier)], sample_terms(), 0).unwrap();
		assert!(matches!(c.status, ContractStatus::Draft));
		let active = c.activate();
		assert!(matches!(active.status, ContractStatus::Active));
		assert_ne!(active.hash, [0u8;32]);
	}
}
