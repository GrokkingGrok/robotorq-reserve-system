//! Batch Processing Error Types
//!
//! This module defines error types related to batch processing operations in the
//! RoboTorq Reserve System. Batch errors handle violations of system invariants
//! during the processing of ore batches, token collections, and other batched operations.
//!
//! # Batch Invariants
//!
//! The system maintains strict invariants about batch sizes and contents to ensure
//! economic integrity and prevent system inconsistencies. These errors are raised
//! when those invariants are violated.
//!
//! # Common Batch Operations
//!
//! - Ore batch aggregation (3,600 JouleTorqOre units per TokenTorqIngot)
//! - Token batch processing (1,000 TokenTorqIngots per RoboTorq Certificate)
//! - Merkle tree batch construction

use thiserror::Error;

/// Errors that occur during batch processing operations.
///
/// These errors represent violations of batch-related invariants in the RoboTorq
/// system, ensuring that all batched operations maintain economic and cryptographic
/// integrity.
#[derive(Debug, Error)]
pub enum BatchError {
    /// The batch size does not match the expected invariant requirements.
    ///
    /// This error occurs when a batch contains an incorrect number of items,
    /// violating the system's economic invariants. For example, a TokenTorqIngot
    /// batch must contain exactly 3,600 JouleTorqOre units.
    ///
    /// # Fields
    /// - `actual`: The actual size of the batch
    /// - `expected`: The expected size according to system invariants
    ///
    /// # Examples
    /// ```rust
    /// # use commons::util::error::batch_error::BatchError;
    /// let error = BatchError::BatchSize { actual: 3500, expected: 3600 };
    /// if let BatchError::BatchSize { actual, expected } = error {
    ///     assert_eq!(actual, 3500);
    ///     assert_eq!(expected, 3600);
    /// }
    /// ```
    #[error("Batch size violates invariant: actual {actual}, expected {expected}")]
    BatchSize { actual: usize, expected: usize },

    /// Attempted to process an empty batch.
    ///
    /// Empty batches are not allowed as they violate the minimum batch size
    /// requirements and would result in invalid cryptographic operations.
    ///
    /// # Causes
    /// - No items provided for batching
    /// - All items filtered out during preprocessing
    /// - Invalid batch construction logic
    #[error("Empty batch")]
    EmptyBatch,
}