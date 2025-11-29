use serde::{Serialize, Deserialize};
use crate::types::ids::{RobotId, UnmappedOreBatchId};
use crate::util::error::InvariantError;
use crate::util::error::batch_error::BatchError;
use crate::util::hashing::hash_struct;
use crate::util::schema::UNMAPPED_ORE_BATCH_SCHEMA_VERSION;
use crate::util::timekeeping::now;
use crate::types::Token;

// Unmapped ore batch: robot output of tokens with per-token joule counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmappedOreBatch {
    pub id: UnmappedOreBatchId,
    pub robot_id: RobotId,
    pub schema_version: u32,
    pub tokens: Vec<Token>,
    pub captured_at_ms: i128,
    pub hash: [u8; 32],
}

impl UnmappedOreBatch {
    pub fn new(robot_id: RobotId, tokens: Vec<Token>) -> Result<Self, InvariantError> {
        if tokens.is_empty() { return Err(InvariantError::from(BatchError::EmptyBatch)); }
        let provisional = Self {
            id: UnmappedOreBatchId::new(),
            robot_id,
            schema_version: UNMAPPED_ORE_BATCH_SCHEMA_VERSION,
            tokens,
            captured_at_ms: now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i128,
            hash: [0u8;32],
        };
        let hash = hash_struct(&provisional);
        Ok(Self { hash, ..provisional })
    }
}
