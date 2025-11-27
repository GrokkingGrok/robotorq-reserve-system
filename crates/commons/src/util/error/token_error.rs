use thiserror::Error;

// Token-related invariants and validation errors.
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Energy value must be positive: {0}")] NegativeEnergy(f64),
    #[error("Joule count must be > 0: {0}")] ZeroJoules(u32),
}