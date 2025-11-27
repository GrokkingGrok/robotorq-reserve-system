use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("logging initialization failed: {0}")]
    InitFailed(String),
}

impl From<&str> for LoggingError {
    fn from(s: &str) -> Self { Self::InitFailed(s.to_string()) }
}

impl From<String> for LoggingError {
    fn from(s: String) -> Self { Self::InitFailed(s) }
}

impl From<Box<dyn std::error::Error>> for LoggingError {
    fn from(e: Box<dyn std::error::Error>) -> Self { Self::InitFailed(e.to_string()) }
}
