use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub enabled: bool,
    pub speedup: f64,
    pub energy_variance: f64,
    pub failure_rate: f64,
    pub network_latency_ms: u64,
    pub random_seed: Option<u64>,
}