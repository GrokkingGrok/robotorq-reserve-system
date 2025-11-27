use serde::{Serialize, Deserialize};
use crate::{RobotId, ContractId, UnmappedOreBatchId, InvariantError, BatchError};
use crate::hashing::hash_struct;
use crate::timekeeper::now;
use crate::token::Token;

// Unmapped ore batch: robot output of tokens with per-token joule counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmappedOreBatch {
    pub id: UnmappedOreBatchId,
    pub contract_id: ContractId,
    pub robot_id: RobotId,
    pub schema_version: u32,
    pub tokens: Vec<Token>,
    pub captured_at_ms: i128,
    pub hash: [u8; 32],
}

impl UnmappedOreBatch {
    pub fn new(contract_id: ContractId, robot_id: RobotId, tokens: Vec<Token>) -> Result<Self, InvariantError> {
        if tokens.is_empty() { return Err(InvariantError::from(BatchError::EmptyBatch)); }
        let provisional = Self {
            id: UnmappedOreBatchId::new(),
            contract_id,
            robot_id,
            schema_version: 1,
            tokens,
            captured_at_ms: now().unix_timestamp_nanos() / 1_000_000,
            hash: [0u8;32],
        };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }
}
