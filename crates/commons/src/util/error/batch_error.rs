use thiserror::Error;


// Batch-related invariants across ore/tokens.
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("Batch size violates invariant: actual {actual}, expected {expected}")] BatchSize { actual: usize, expected: usize },
    #[error("Empty batch")] EmptyBatch,
}