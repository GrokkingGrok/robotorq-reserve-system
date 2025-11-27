use thiserror::Error;

// Gateway-specific errors for robot-gateway service.
#[derive(Debug, Error)]
pub enum RobotGatewayError {
    #[error("Unknown robot id or robot not registered")] UnknownRobotId,
    #[error("Robot already registered")] AlreadyRegistered,
    #[error("metrics not configured for gateway")] MetricsNotConfigured,
}