//! Robotic entities that perform work and generate `JouleTorqOre`.
//!
//! Robots are the primary work-performing agents in the `RoboTorq` Reserve System.
//! They execute contracts by performing physical labor, generating tokens that
//! represent measurable work output. Each robot has defined throughput ratings
//! and operational status, forming the foundation of the system's work proof
//! mechanism.

pub mod printer;

use crate::types::ids::{ContractId, RobotId};
use crate::util::error::robot_error::RobotError;
use crate::util::schema::ROBOT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

/// A robotic work-performing entity in the `RoboTorq` network.
///
/// Robots are the fundamental producers of value in the `RoboTorq` Reserve System.
/// They execute contracts by performing physical work, generating `JouleTorqOre`
/// tokens that represent measurable robotic labor. Each robot has defined
/// performance characteristics and operational constraints that ensure
/// predictable and verifiable work output.
///
/// # Economic Role
/// - Primary generators of `JouleTorqOre` tokens through physical work
/// - Execute contracts defining work requirements and compensation
/// - Provide verifiable work proof through token generation and batching
/// - Enable distributed work execution across the robotic network
///
/// # Operational Characteristics
/// Robots have defined throughput ratings for both token production and
/// energy consumption, ensuring predictable performance and economic
/// calculations. They maintain operational status and contract assignments
/// to coordinate work across the distributed system.
///
/// # Fields
/// - `id`: Unique identifier for this robot
/// - `name`: Human-readable identifier for operational purposes
/// - `token_throughput_rating`: Maximum tokens this robot can produce per second
/// - `joule_throughput_rating`: Maximum joules this robot can consume per second (watts)
/// - `is_working`: Current operational status (true = active, false = idle)
/// - `active_contract`: ID of the contract this robot is currently executing
/// - `schema_version`: Version of the robot schema for compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Robot {
    /// Unique identifier of the robot.
    pub id: RobotId,
    /// Human-readable identifier for operational use.
    pub name: String,
    /// Rated tokens the robot can produce per second.
    pub token_throughput_rating: u32,
    /// Rated joules the robot can consume per second (watts).
    pub joule_throughput_rating: u32,
    /// Current operational status (true = working, false = idle).
    pub is_working: bool,
    /// Currently active contract identifier.
    pub active_contract: ContractId,
    /// Schema version for compatibility and migrations.
    pub schema_version: u32,
}

impl Robot {
    /// Creates a new robot with validated parameters.
    ///
    /// This constructor ensures that all robot parameters meet system invariants:
    /// - Name must not be empty or whitespace-only
    /// - Token throughput must be positive (robots must produce tokens)
    /// - Joule throughput must be positive (robots must consume energy)
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the robot
    /// * `name` - Human-readable name (must not be empty/whitespace)
    /// * `token_throughput_rating` - Maximum tokens per second (must be > 0)
    /// * `joule_throughput_rating` - Maximum joules per second (must be > 0)
    /// * `is_working` - Initial operational status
    /// * `active_contract` - Contract this robot will execute
    ///
    /// # Returns
    /// Returns a `Result` containing the new robot or a `RobotError` if validation fails.
    ///
    /// # Errors
    /// - `RobotError::InvalidRobotName` if name is empty or whitespace-only
    /// - `RobotError::ZeroTokenThroughput` if `token_throughput_rating` is 0
    /// - `RobotError::ZeroJouleThroughput` if `joule_throughput_rating` is 0
    ///
    /// # Examples
    /// ```rust
    /// # use commons::types::ids::{RobotId, ContractId};
    /// # use commons::types::robot::Robot;
    /// let robot_id = RobotId::new();
    /// let contract_id = ContractId::new();
    ///
    /// let robot = Robot::new(
    ///     robot_id,
    ///     "KLP-01",
    ///     5,    // 5 tokens per second
    ///     500,  // 500 watts
    ///     true, // initially working
    ///     contract_id
    /// ).unwrap();
    ///
    /// assert_eq!(robot.name, "KLP-01");
    /// assert_eq!(robot.token_throughput_rating, 5);
    /// assert_eq!(robot.joule_throughput_rating, 500);
    /// ```
    pub fn new(
        id: RobotId,
        name: impl Into<String>,
        token_throughput_rating: u32,
        joule_throughput_rating: u32,
        is_working: bool,
        active_contract: ContractId,
    ) -> Result<Self, RobotError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(RobotError::InvalidRobotName);
        }
        if token_throughput_rating == 0 {
            return Err(RobotError::ZeroTokenThroughput);
        }
        if joule_throughput_rating == 0 {
            return Err(RobotError::ZeroJouleThroughput);
        }

        Ok(Self {
            id,
            name,
            token_throughput_rating,
            joule_throughput_rating,
            is_working,
            active_contract,
            schema_version: ROBOT_SCHEMA_VERSION,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ids::RobotId;

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
        matches!(err, RobotError::InvalidRobotName);
    }

    #[test]
    fn robot_new_rejects_zero_token_rate() {
        let id = RobotId::new();
        let err = Robot::new(id, "A", 0, 500, true, ContractId::new()).unwrap_err();
        matches!(err, RobotError::ZeroTokenThroughput);
    }

    #[test]
    fn robot_new_rejects_non_positive_watts() {
        let id = RobotId::new();
        let err = Robot::new(id, "A", 5, 0, true, ContractId::new()).unwrap_err();
        matches!(err, RobotError::ZeroJouleThroughput);
    }
}
