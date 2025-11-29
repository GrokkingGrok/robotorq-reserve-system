use serde::{Deserialize, Serialize};

/// Configuration parameters for simulatioon mode.
/// 
/// The RoboTorq Reserve System Simulator cannot be legally activated
/// from this repo. 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub enabled: bool,
    pub random_seed: Option<u64>,
    
    // Timing and performance simulation parameters
    pub in_simulation_hours: Option<u64>,       // 8760 hours = 1 year in sim time
    pub speedup: Option<f64>,                   // e.g., 100.0 = 100x real-time speed -> sleep timer / 100.0 -> 88 real hours = 1 year in-sim
    pub network_latency_ms: Option<u64>,        // simulated network latency in milliseconds

    
    // Robot Gateway specific simulation parameters
    pub robot_gateway_robot_initial_count: Option<usize>,
    pub robot_gateway_robot_count_growth_rate: Option<f64>,
    pub robot_gateway_max_robot_count: Option<usize>,

    /* 
    pub robot_gateway_average_token_throughput: u32,
    pub robot_gateway_variance_token_throughput: u32,
    pub robot_gateway_average_joule_throughput: u32,
    pub robot_gateway_variance_joule_throughput: u32,
    */
}

impl Default for Simulation {
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