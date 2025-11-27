pub mod types;
pub mod util;

pub mod merkle;
pub mod services;

// Ensure the sim feature cannot be enabled without the external gate crate.
// The dependency is required by Cargo when feature=sim; this import guarantees linkage.
#[cfg(feature = "sim")]
mod _sim_feature_guard {
	use robotorq_sim_gate as _;
}

pub use types::*;
// Avoid glob re-exports to prevent namespace collisions; use module paths.
