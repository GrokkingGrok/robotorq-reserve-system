use thiserror::Error;

// Configuration parsing/validation errors (reserved for future use).
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")] Invalid(String),
}