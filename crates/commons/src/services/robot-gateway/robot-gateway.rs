use std::path::Path;
use crate::{RobotId, ContractId, Token, UnmappedOreBatch, InvariantError};

/// Gateway coordinating one or more robots for batch capture.
/// Maintains a list of registered robot IDs and provides helpers to produce unmapped ore batches.
pub struct RobotGateway {
    robots: Vec<RobotId>,
}

impl RobotGateway {
    /// Create gateway from an iterator of robot IDs (deduplicated, order preserved by first occurrence).
    pub fn new<I: IntoIterator<Item = RobotId>>(ids: I) -> Self {
        let mut robots: Vec<RobotId> = Vec::new();
        for id in ids { if !robots.contains(&id) { robots.push(id); } }
        Self { robots }
    }

    /// Convenience for single robot.
    pub fn single(robot_id: RobotId) -> Self { Self { robots: vec![robot_id] } }

    /// Load from a config file path (stub: returns one new robot for now).
    pub fn from_config(_path: &Path) -> Result<Self, String> {
        Ok(Self::single(RobotId::new()))
    }

    /// Register an additional robot (no-op if already present).
    pub fn register_robot(&mut self, robot_id: RobotId) {
        if !self.robots.contains(&robot_id) { self.robots.push(robot_id); }
    }

    /// All registered robots.
    pub fn robots(&self) -> &[RobotId] { &self.robots }

    /// Capture a batch for a specific robot id, ensuring the robot is registered.
    pub fn capture_unmapped_batch_for(&self, robot_id: RobotId, contract_id: ContractId, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        if !self.robots.contains(&robot_id) {
            return Err(InvariantError::Robot(crate::RobotError::InvalidRobotName)); // placeholder error variant reuse
        }
        UnmappedOreBatch::new(contract_id, robot_id, tokens)
    }

    /// Capture a batch using the first registered robot (returns error if none).
    pub fn capture_unmapped_batch_any(&self, contract_id: ContractId, tokens: Vec<Token>) -> Result<UnmappedOreBatch, InvariantError> {
        let robot_id = *self.robots.get(0).ok_or_else(|| InvariantError::Robot(crate::RobotError::InvalidRobotName))?;
        UnmappedOreBatch::new(contract_id, robot_id, tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_single_robot_batch() {
        let gw = RobotGateway::single(RobotId::new());
        let t1 = Token::map(42).unwrap();
        let batch = gw.capture_unmapped_batch_any(ContractId::new(), vec![t1]).unwrap();
        assert_eq!(batch.tokens.len(), 1);
    }

    #[test]
    fn gateway_multi_robot_specific_capture() {
        let r1 = RobotId::new();
        let r2 = RobotId::new();
        let gw = RobotGateway::new(vec![r1, r2]);
        let t = Token::map(10).unwrap();
        let batch = gw.capture_unmapped_batch_for(r2, ContractId::new(), vec![t]).unwrap();
        assert_eq!(batch.tokens.len(), 1);
    }

    #[test]
    fn gateway_unknown_robot_rejected() {
        let r1 = RobotId::new();
        let gw = RobotGateway::single(r1);
        let r_unknown = RobotId::new();
        let t = Token::map(5).unwrap();
        let err = gw.capture_unmapped_batch_for(r_unknown, ContractId::new(), vec![t]).unwrap_err();
        // Using RobotError::InvalidRobotName as placeholder classification.
        matches!(err, crate::InvariantError::Robot(crate::RobotError::InvalidRobotName));
    }
}