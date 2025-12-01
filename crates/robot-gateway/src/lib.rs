pub mod metrics;
use std::path::Path;
use commons::types::ids::RobotId;
use commons::types::token::Token;
use commons::types::ore::UnmappedOreBatch;
use commons::util::error::InvariantError;
use commons::util::error::robot_gateway_error::RobotGatewayError;
use crate::metrics::RobotGatewayMetrics;
use commons::services::http::RoboTorqService;
use commons::util::config::RoboTorqConfig;

/// Gateway coordinating one or more robots for batch capture.
/// Maintains a list of registered robot IDs and provides helpers to produce unmapped ore batches.
pub struct RobotGateway {
    robots: Vec<RobotId>,
    metrics: Option<RobotGatewayMetrics>,
}

impl RobotGateway {
    /// Create gateway from an iterator of robot IDs (deduplicated, order preserved by first occurrence).
    pub fn new<I: IntoIterator<Item = RobotId>>(ids: I) -> Self {
        let mut robots: Vec<RobotId> = Vec::new();
        for id in ids { if !robots.contains(&id) { robots.push(id); } }
        Self { robots, metrics: None }
    }

    /// Convenience for single robot.
    #[must_use]
    pub fn single(robot_id: RobotId) -> Self { Self { robots: vec![robot_id], metrics: None } }

    /// Attach metrics to this gateway (builder-style). Updates registered robots gauge immediately.
    #[must_use]
    pub fn with_metrics(mut self, metrics: RobotGatewayMetrics) -> Self {
        let count = self.robots.len();
        metrics.set_registered(count);
        self.metrics = Some(metrics);
        self
    }

    /// Load from a config file path (stub: returns one new robot for now).
    ///
    /// # Errors
    ///
    /// This function currently does not return any errors but may in the future
    /// when actual configuration file parsing is implemented.
    #[must_use]
    pub fn from_config(_path: &Path) -> Result<Self, String> {
        Ok(Self::single(RobotId::new()))
    }

    /// Register an additional robot (no-op if already present).
    pub fn register_robot(&mut self, robot_id: RobotId) {
        if !self.robots.contains(&robot_id) {
            self.robots.push(robot_id);
            if let Some(m) = &self.metrics { m.set_registered(self.robots.len()); }
        }
    }

    /// All registered robots.
    pub fn robots(&self) -> &[RobotId] { &self.robots }

    /// Capture an unmapped batch for a specific robot.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::UnknownRobotId)` if the
    /// specified `robot_id` is not registered with this gateway.
    pub fn capture_unmapped_batch_for(&self, robot_id: RobotId, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        if !self.robots.contains(&robot_id) {
            if let Some(m) = &self.metrics { m.inc_rejected(); }
            return Err(InvariantError::Gateway(RobotGatewayError::UnknownRobotId));
        }
        let res = UnmappedOreBatch::new(robot_id, tokens);
        if res.is_ok() && let Some(m) = &self.metrics { m.inc_captured(); }
        res
    }

    /// Capture a batch using the first registered robot (returns error if none).
    ///
    /// # Errors
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::UnknownRobotId)` if no
    /// robots are registered with this gateway.
    pub fn capture_unmapped_batch_any(&self, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        let robot_id = *self.robots.first().ok_or_else(|| {
            if let Some(m) = &self.metrics { m.inc_rejected(); }
            InvariantError::Gateway(RobotGatewayError::UnknownRobotId)
        })?;
        let res = UnmappedOreBatch::new(robot_id, tokens);
        if res.is_ok() && let Some(m) = &self.metrics { m.inc_captured(); }
        res
    }
}

impl RoboTorqService for RobotGateway {
    /// Perform a health check on the robot gateway service.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::UnknownRobotId)` if no
    /// robots are registered with this gateway.
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::MetricsNotConfigured)` if
    /// metrics are not configured for this gateway.
    fn health_check(&self) -> Result<String, InvariantError> {
        // Basic health check: ensure we have robots and metrics configured
        if self.robots.is_empty() {
            return Err(InvariantError::Gateway(RobotGatewayError::UnknownRobotId));
        }
        if self.metrics.is_none() {
            return Err(InvariantError::Gateway(RobotGatewayError::MetricsNotConfigured));
        }
        Ok(format!("RobotGateway healthy: {} robots registered", self.robots.len()))
    }
    
    fn export_metrics(&self) -> String {
        self.metrics.as_ref()
            .map(|m| m.get_handler().export_text())
            .unwrap_or_else(|| "# No metrics configured\n".to_string())
    }

    fn handle_request(&self, _path: &str, _method: &str) -> Option<Result<String, InvariantError>> {
        None
    }

    /// Initialize the robot gateway service with the provided configuration.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::InvalidConfiguration)` if
    /// the robot_gateway_port in the configuration is set to 0.
    async fn initialize(&mut self, config: &RoboTorqConfig) -> Result<(), InvariantError> {
        // Validate configuration for robot gateway
        if config.ports.robot_gateway_port == 0 {
            return Err(InvariantError::Gateway(RobotGatewayError::InvalidConfiguration(
                "robot_gateway_port cannot be 0".to_string()
            )));
        }

        // Initialize metrics if not already present
        if self.metrics.is_none() {
            let metrics = RobotGatewayMetrics::new("robot_gateway");
            self.metrics = Some(metrics);
        }

        // Update metrics with current robot count
        if let Some(m) = &self.metrics {
            m.set_registered(self.robots.len());
        }

        tracing::info!(
            component = "robot-gateway",
            robot_count = self.robots.len(),
            port = config.ports.robot_gateway_port,
            "RobotGateway initialized successfully"
        );

        Ok(())
    }

    /// Start the robot gateway service.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::InvalidConfiguration)` if
    /// no robots are registered with this gateway.
    ///
    /// Returns `InvariantError::Gateway(RobotGatewayError::MetricsNotConfigured)` if
    /// metrics are not configured for this gateway.
    async fn start(&self) -> Result<(), InvariantError> {
        // Validate that we're properly configured
        if self.robots.is_empty() {
            return Err(InvariantError::Gateway(RobotGatewayError::InvalidConfiguration(
                "No robots registered".to_string()
            )));
        }

        if self.metrics.is_none() {
            return Err(InvariantError::Gateway(RobotGatewayError::MetricsNotConfigured));
        }

        // Log that we're starting
        tracing::info!(
            component = "robot-gateway",
            robot_count = self.robots.len(),
            "RobotGateway started and ready to accept requests"
        );

        Ok(())
    }

    /// Stop the robot gateway service gracefully.
    ///
    /// # Errors
    ///
    /// This method currently does not return any errors but may in the future
    /// when background tasks or connections need to be shut down.
    async fn stop(&self) -> Result<(), InvariantError> {
        // Log that we're stopping
        tracing::info!(
            component = "robot-gateway",
            "RobotGateway stopping - no longer accepting new requests"
        );

        // For now, this is a no-op since RobotGateway doesn't maintain
        // background tasks or connections that need to be stopped.
        // In the future, if we add connection pools, background workers,
        // or other resources, they would be gracefully shut down here.

        Ok(())
    }

    /// Shut down the robot gateway service and release all resources.
    ///
    /// # Errors
    ///
    /// This method currently does not return any errors but may in the future
    /// when resources need explicit cleanup.
    async fn shutdown(&self) -> Result<(), InvariantError> {
        // Log final shutdown
        tracing::info!(
            component = "robot-gateway",
            "RobotGateway shutdown complete - all resources released"
        );

        // For now, this is a no-op since RobotGateway doesn't maintain
        // resources that need explicit cleanup.
        // In the future, if we add file handles, network connections,
        // or other resources, they would be cleaned up here.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_single_robot_batch() {
        let gw = RobotGateway::single(RobotId::new());
        let t1 = Token::new(42).unwrap();
        let batch = gw.capture_unmapped_batch_any(vec![t1]).unwrap();
        assert_eq!(batch.tokens.len(), 1);
    }

    #[test]
    fn gateway_multi_robot_specific_capture() {
        let r1 = RobotId::new();
        let r2 = RobotId::new();
        let gw = RobotGateway::new(vec![r1, r2]);
        let t = Token::new(10).unwrap();
        let batch = gw.capture_unmapped_batch_for(r2,  vec![t]).unwrap();
        assert_eq!(batch.tokens.len(), 1);
    }

    #[test]
    fn gateway_unknown_robot_rejected() {
        let r1 = RobotId::new();
        let gw = RobotGateway::single(r1);
        let r_unknown = RobotId::new();
        let t = Token::new(5).unwrap();
        let err = gw.capture_unmapped_batch_for(r_unknown, vec![t]).unwrap_err();
        matches!(err, InvariantError::Gateway(RobotGatewayError::UnknownRobotId));
    }
}
