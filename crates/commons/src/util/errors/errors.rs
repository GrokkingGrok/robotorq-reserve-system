use thiserror::Error;

// Token-related invariants and validation errors.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Energy value must be positive: {0}")] NegativeEnergy(f64),
    #[error("Joule count must be > 0: {0}")] ZeroJoules(u32),
}

// Batch-related invariants across ore/tokens.
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("Batch size violates invariant: actual {actual}, expected {expected}")] BatchSize { actual: usize, expected: usize },
    #[error("Empty batch")] EmptyBatch,
}

// Robot-specific validation errors.
#[derive(Debug, Error)]
pub enum RobotError {
    #[error("Robot name must be non-empty")] InvalidRobotName,
    #[error("Token throughput per second must be > 0")] ZeroTokenThroughput,
    #[error("Joule throughput (watts) must be > 0")] ZeroJouleThroughput,
}

// Configuration parsing/validation errors (reserved for future use).
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")] Invalid(String),
}

// TripleTorq-Validation errors.
#[derive(Debug, Error)]
pub enum TripleTorqError {
    #[error("TripleTorq cannot be negative")] NegativeTripleTorqError,
    #[error("TokenTorq cannot be >= 1000")] TokenTorqRolloverError,
    #[error("JouleTorq cannot be >= 3600")] JouleTorqRolloverError,
}

// Unified invariant error type used across constructors and validators.
#[derive(Debug, Error)]
pub enum InvariantError {
    #[error(transparent)] Token(#[from] TokenError),
    #[error(transparent)] Batch(#[from] BatchError),
    #[error(transparent)] Robot(#[from] RobotError),
    #[error(transparent)] Config(#[from] ConfigError),
    #[error(transparent)] TripleTorq(#[from] TripleTorqError),
}
