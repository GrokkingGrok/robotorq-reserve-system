pub mod token_error;
pub mod batch_error;
pub mod robot_error;
pub mod robot_gateway_error;
pub mod config_error;
pub mod triple_torq_error;
pub mod logging_error;
pub mod prometheus_error;

use thiserror::Error;
use token_error::TokenError;
use batch_error::BatchError;
use robot_error::RobotError;
use robot_gateway_error::RobotGatewayError;
use config_error::ConfigError;
use triple_torq_error::TripleTorqError;
use logging_error::LoggingError;
use prometheus_error::PrometheusError;

// Unified invariant error type used across constructors and validators.
#[derive(Debug, Error)]
pub enum InvariantError {
    #[error(transparent)] Token(#[from] TokenError),
    #[error(transparent)] Batch(#[from] BatchError),
    #[error(transparent)] Robot(#[from] RobotError),
    #[error(transparent)] Gateway(#[from] RobotGatewayError),
    #[error(transparent)] Config(#[from] ConfigError),
    #[error(transparent)] TripleTorq(#[from] TripleTorqError),
    #[error(transparent)] Logging(#[from] LoggingError),
    #[error(transparent)] Metrics(#[from] PrometheusError),
}
