pub mod metrics;
use std::path::Path;
use crate::types::ids::RobotId;
use crate::types::token::Token;
use crate::types::ore::UnmappedOreBatch;
use crate::services::http::{HttpService, HttpEndpoint, HttpServerConfig, start_basic_http_server_with_config};
use crate::util::error::InvariantError;
use crate::util::error::robot_gateway_error::RobotGatewayError;
use crate::services::robot_gateway::metrics::RobotGatewayMetrics;

/// Gateway coordinating one or more robots for batch capture.
/// Maintains a list of registered robot IDs and provides helpers to produce unmapped ore batches.
pub struct RobotGateway {
    robots: Vec<RobotId>,
    metrics: Option<RobotGatewayMetrics>,
    http_service: Option<HttpService>,
    health_endpoint: Option<HttpEndpoint>,
    metrics_endpoint: Option<HttpEndpoint>,

}

impl RobotGateway {
    /// Create gateway from an iterator of robot IDs (deduplicated, order preserved by first occurrence).
    pub fn new<I: IntoIterator<Item = RobotId>>(ids: I) -> Self {
        let mut robots: Vec<RobotId> = Vec::new();
        for id in ids { if !robots.contains(&id) { robots.push(id); } }
        Self { robots, metrics: None, http_service: None, health_endpoint: None, metrics_endpoint: None }
    }

    /// Convenience for single robot.
    pub fn single(robot_id: RobotId) -> Self { Self { robots: vec![robot_id], metrics: None, http_service: None, health_endpoint: None, metrics_endpoint: None } }

    /// Attach metrics to this gateway (builder-style). Updates registered robots gauge immediately.
    pub fn with_metrics(mut self, metrics: RobotGatewayMetrics) -> Self {
        let count = self.robots.len();
        metrics.set_registered(count);
        self.metrics = Some(metrics);
        self
    }

    /// Attach HTTP config to this gateway (builder-style).
    pub fn with_http_config(mut self, cfg: HttpServerConfig) -> Self {
        self.http_service = Some(cfg.service);
        self.health_endpoint = Some(cfg.health);
        self.metrics_endpoint = Some(cfg.metrics);
        self
    }

    /// Load from a config file path (stub: returns one new robot for now).
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

    pub fn capture_unmapped_batch_for(&self, robot_id: RobotId, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        if !self.robots.contains(&robot_id) {
            if let Some(m) = &self.metrics { m.inc_rejected(); }
            return Err(InvariantError::Gateway(RobotGatewayError::UnknownRobotId));
        }
        let res = UnmappedOreBatch::new(robot_id, tokens);
        if res.is_ok() { if let Some(m) = &self.metrics { m.inc_captured(); } }
        res
    }

    /// Capture a batch using the first registered robot (returns error if none).
    pub fn capture_unmapped_batch_any(&self, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        let robot_id = *self.robots.get(0).ok_or_else(|| {
            if let Some(m) = &self.metrics { m.inc_rejected(); }
            InvariantError::Gateway(RobotGatewayError::UnknownRobotId)
        })?;
        let res = UnmappedOreBatch::new(robot_id, tokens);
        if res.is_ok() { if let Some(m) = &self.metrics { m.inc_captured(); } }
        res
    }

    /// Start the HTTP server using configured endpoints and metrics.
    /// Falls back to local defaults if HTTP config not provided.
    pub fn start_http_server(&self) -> Result<std::thread::JoinHandle<()>, InvariantError> {
        let metrics = self.metrics.as_ref().ok_or_else(|| InvariantError::Gateway(RobotGatewayError::MetricsNotConfigured))?;

        let service = self.http_service.clone().unwrap_or_else(|| HttpService::new("127.0.0.1", 9090));
        let health = self.health_endpoint.clone().unwrap_or_else(|| HttpEndpoint::new("/health"));
        let metrics_ep = self.metrics_endpoint.clone().unwrap_or_else(|| HttpEndpoint::new("/metrics"));

        let cfg = HttpServerConfig::new(service, health, metrics_ep);
        start_basic_http_server_with_config(metrics.get_handler(), cfg)
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
