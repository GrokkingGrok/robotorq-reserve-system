//! Raw JouleTorqOre batches produced by robots.
//!
//! Unmapped ore batches represent the initial output from robots performing work.
//! These batches contain raw tokens that haven't yet been aggregated into higher-level
//! structures like TokenTorqIngots or RoboTorqCertificates. They serve as the
//! foundation of the work proof chain in the RoboTorq Reserve System.
//!
//! Each batch is cryptographically sealed with a hash and timestamped to ensure
//! immutability and temporal ordering of work performed.

use serde::{Serialize, Deserialize};
use crate::types::ids::{RobotId, UnmappedOreBatchId};
use crate::util::error::InvariantError;
use crate::util::error::batch_error::BatchError;
use crate::util::hashing::hash_struct;
use crate::util::schema::UNMAPPED_ORE_BATCH_SCHEMA_VERSION;
use crate::util::timekeeping::now;
use crate::types::Token;

/// A batch of raw JouleTorqOre tokens produced by a robot.
///
/// Unmapped ore batches are the atomic units of work proof in the RoboTorq system.
/// They contain tokens representing individual joules of robotic labor that haven't
/// yet been aggregated into higher-level economic structures. Each batch is
/// cryptographically sealed and timestamped to maintain the integrity of the
/// work proof chain.
///
/// # Economic Role
/// - Foundation of the JouleTorqOre → TokenTorqIngot → RoboTorqCertificate hierarchy
/// - Provides raw work proof data for batching and aggregation operations
/// - Enables tracking of work performed by individual robots over time
///
/// # Fields
/// - `id`: Unique identifier for this batch
/// - `robot_id`: The robot that produced this batch
/// - `schema_version`: Version of the batch schema for compatibility
/// - `tokens`: Vector of individual tokens representing work performed
/// - `captured_at_ms`: Timestamp when the batch was captured (milliseconds since Unix epoch)
/// - `hash`: Cryptographic hash of the batch contents for integrity verification
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
    /// Creates a new unmapped ore batch from a robot's token output.
    ///
    /// This constructor validates that the batch contains at least one token
    /// (empty batches are not allowed as they represent no work performed).
    /// The batch is timestamped with the current time and cryptographically
    /// sealed with a hash to ensure immutability.
    ///
    /// # Arguments
    /// * `robot_id` - The ID of the robot that produced these tokens
    /// * `tokens` - Vector of tokens representing work performed (must not be empty)
    ///
    /// # Returns
    /// Returns a `Result` containing the new batch or an `InvariantError` if validation fails.
    ///
    /// # Errors
    /// Returns `BatchError::EmptyBatch` if the tokens vector is empty.
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::ids::RobotId;
    /// # use commons::types::Token;
    /// # use commons::types::ore::unmapped_ore_batch::UnmappedOreBatch;
    /// let robot_id = RobotId::new();
    /// let tokens = vec![Token::map(100).unwrap()]; // Some work tokens
    ///
    /// let batch = UnmappedOreBatch::new(robot_id, tokens).unwrap();
    /// assert_eq!(batch.robot_id, robot_id);
    /// assert!(!batch.tokens.is_empty());
    /// ```
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
