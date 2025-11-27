#[path = "types/fields/ids/ids.rs"]
pub mod ids;
#[path = "util/timekeeping/timekeeper.rs"]
pub mod timekeeper;
#[path = "util/hashing/hashing.rs"]
pub mod hashing;
#[path = "types/ore/unmapped_ore_batch.rs"]
pub mod ore;
#[path = "types/robot/robot.rs"]
pub mod robot;
#[path = "types/token/token.rs"]
pub mod token;
#[path = "types/contract/contract.rs"]
pub mod contract;
#[path = "types/currency/triple_torq/triple_torq.rs"]
pub mod triple_torq;
#[path = "types/currency/triple_torq/triple_torq_math.rs"]
pub mod triple_torq_math;
#[path = "util/errors/errors.rs"]
pub mod errors;
#[path = "util/config/config.rs"]
pub mod config;
#[path = "util/logging/logging.rs"]
pub mod logging;
#[path = "util/metrics/metrics.rs"]
pub mod metrics;
#[path = "merkle/merkle.rs"]
pub mod merkle;
#[path = "services/robot-gateway/robot-gateway.rs"]
pub mod robot_gateway;

// Ensure the sim feature cannot be enabled without the external gate crate.
// The dependency is required by Cargo when feature=sim; this import guarantees linkage.
#[cfg(feature = "sim")]
mod _sim_feature_guard {
	use robotorq_sim_gate as _;
}

pub use ids::*;
pub use ore::*;
pub use robot::*;
pub use token::*;
pub use contract::*;
pub use triple_torq::*;
pub use triple_torq_math::*;
pub use errors::*;
pub use config::*;
pub use logging::*;
pub use metrics::*;
pub use merkle::*;
pub use robot_gateway::*;
