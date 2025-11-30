//! Simulation Configuration Parameters
//!
//! This module defines configuration parameters for simulation mode operation
//! in the RoboTorq Reserve System. Simulation mode allows testing and development
//! without requiring physical robotic hardware or real economic transactions.
//!
//! # Simulation Architecture
//!
//! The simulation system provides:
//! - **Time Control**: Adjustable time scaling for accelerated testing
//! - **Network Simulation**: Configurable latency and network conditions
//! - **Robot Population**: Dynamic robot count management
//! - **Deterministic Behavior**: Optional seeded random number generation
//!
//! # Time Simulation
//!
//! Time simulation allows compressing real-world time for testing:
//! - `speedup = 100.0` means 1 simulated year takes ~88 real hours
//! - `in_simulation_hours` limits the total simulation duration
//! - Network latency can be added to simulate real-world conditions
//!
//! # Robot Gateway Simulation
//!
//! The robot gateway simulation controls:
//! - Initial robot population size
//! - Growth rate for robot count over time
//! - Maximum robot count limits
//!
//! # Legal Notice
//!
//! The RoboTorq Reserve System Simulator cannot be legally activated
//! from this repository. Simulation mode is provided for development
//! and testing purposes only.

use serde::{Deserialize, Serialize};

/// Configuration parameters for simulation mode.
///
/// This struct contains all parameters that control the behavior of the
/// RoboTorq system when operating in simulation mode. All fields are optional
/// and will use sensible defaults if not specified.
///
/// # Usage Context
///
/// These parameters are only used when the system `Mode` is set to `Simulation`.
/// In production mode, these settings are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    /// Whether simulation mode is enabled.
    ///
    /// This must be `true` for simulation parameters to take effect.
    /// When `false`, the system will not enter simulation mode even
    /// if other simulation parameters are configured.
    pub enabled: bool,

    /// Random seed for deterministic simulation behavior.
    ///
    /// When provided, this seed ensures reproducible simulation results
    /// across multiple runs. Useful for testing and debugging.
    /// If `None`, the simulation will use system entropy for randomness.
    pub random_seed: Option<u64>,

    /// Total duration of the simulation in simulated hours.
    ///
    /// Limits how long the simulation will run. For example, 8760 hours
    /// represents one simulated year. If `None`, the simulation runs
    /// indefinitely until manually stopped.
    pub in_simulation_hours: Option<u64>,

    /// Time acceleration factor for the simulation.
    ///
    /// Controls how fast simulated time passes relative to real time.
    /// For example:
    /// - `1.0` = real-time speed
    /// - `100.0` = 100x speed (1 simulated year ≈ 88 real hours)
    /// - `None` = real-time speed
    pub speedup: Option<f64>,

    /// Simulated network latency in milliseconds.
    ///
    /// Adds artificial delay to network operations to simulate real-world
    /// network conditions. Useful for testing timeout behavior and
    /// performance under network stress.
    pub network_latency_ms: Option<u64>,

    /// Initial number of robots in the simulation.
    ///
    /// Sets the starting population of virtual robots in the robot gateway.
    /// If `None`, uses a default initial count.
    pub robot_gateway_robot_initial_count: Option<usize>,

    /// Growth rate for robot population over time.
    ///
    /// Controls how quickly the robot population increases during simulation.
    /// Expressed as a multiplier per simulation time unit.
    /// If `None`, robot population remains constant.
    pub robot_gateway_robot_count_growth_rate: Option<f64>,

    /// Maximum number of robots allowed in the simulation.
    ///
    /// Caps the robot population growth to prevent unbounded resource usage.
    /// If `None`, no maximum limit is enforced.
    pub robot_gateway_max_robot_count: Option<usize>,

    /*
    pub robot_gateway_average_token_throughput: u32,
    pub robot_gateway_variance_token_throughput: u32,
    pub robot_gateway_average_joule_throughput: u32,
    pub robot_gateway_variance_joule_throughput: u32,
    */
}

impl Default for Simulation {
    /// Creates a default simulation configuration with all features disabled.
    ///
    /// The default configuration disables simulation mode and sets all
    /// optional parameters to `None`, resulting in real-time operation
    /// with default system parameters.
    fn default() -> Self {
        Self {
            enabled: false,
            random_seed: None,
            in_simulation_hours: None,
            speedup: None,
            network_latency_ms: None,
            robot_gateway_robot_initial_count: None,
            robot_gateway_robot_count_growth_rate: None,
            robot_gateway_max_robot_count: None,
        }
    }
}