use serde::{Serialize, Deserialize};
use crate::{RobotId, InvariantError, RobotError, ContractId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Robot {
    pub id: RobotId,
    pub name: String,                  // human-readable identifier
    pub token_throughput_rating: u32,  // rated tokens per second
    pub joule_throughput_rating: u32,  // rated joules per second (watts)
    pub is_working: bool,              // operational status
    pub active_contract: ContractId,   // current active contract
}

impl Robot {
    pub fn new(
        id: RobotId,
        name: impl Into<String>,
        token_throughput_rating: u32,
        joule_throughput_rating: u32,
        is_working: bool,
        active_contract: ContractId,
    ) -> Result<Self, InvariantError> {
        let name = name.into();
        if name.trim().is_empty() { return Err(InvariantError::from(RobotError::InvalidRobotName)); }
        if token_throughput_rating == 0 { return Err(InvariantError::from(RobotError::ZeroTokenThroughput)); }
        if joule_throughput_rating == 0 { return Err(InvariantError::from(RobotError::ZeroJouleThroughput)); }

        Ok(Self { id, name, token_throughput_rating, joule_throughput_rating, is_working, active_contract })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RobotId;

    #[test]
    fn robot_new_ok() {
        let id = RobotId::new();
        let r = Robot::new(id, "KLP-01", 5, 500, true, ContractId::new()).unwrap();
        assert_eq!(r.name, "KLP-01");
        assert_eq!(r.token_throughput_rating, 5);
        assert!(r.joule_throughput_rating > 0);
    }

    #[test]
    fn robot_new_rejects_empty_name() {
        let id = RobotId::new();
        let err = Robot::new(id, "  ", 5, 500, true, ContractId::new()).unwrap_err();
        matches!(err, InvariantError::Robot(RobotError::InvalidRobotName));
    }

    #[test]
    fn robot_new_rejects_zero_token_rate() {
        let id = RobotId::new();
        let err = Robot::new(id, "A", 0, 500, true, ContractId::new()).unwrap_err();
        matches!(err, InvariantError::Robot(RobotError::ZeroTokenThroughput));
    }

    #[test]
    fn robot_new_rejects_non_positive_watts() {
        let id = RobotId::new();
        let err = Robot::new(id, "A", 5, 0, true, ContractId::new()).unwrap_err();
        matches!(err, InvariantError::Robot(RobotError::ZeroJouleThroughput));
    }
}