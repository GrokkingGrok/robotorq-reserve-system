use thiserror::Error;

// Robot-specific validation errors.
#[derive(Debug, Error)]
pub enum RobotError {
    #[error("Robot name must be non-empty")] InvalidRobotName,
    #[error("Token throughput per second must be > 0")] ZeroTokenThroughput,
    #[error("Joule throughput (watts) must be > 0")] ZeroJouleThroughput,
}
