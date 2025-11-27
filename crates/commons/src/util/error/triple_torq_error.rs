use thiserror::Error;


// TripleTorq-Validation errors.
#[derive(Debug, Error)]
pub enum TripleTorqError {
    #[error("TripleTorq cannot be negative")] NegativeTripleTorqError,
    #[error("TokenTorq cannot be >= 1000")] TokenTorqRolloverError,
    #[error("JouleTorq cannot be >= 3600")] JouleTorqRolloverError,
}