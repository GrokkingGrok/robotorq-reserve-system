use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestration {
    pub mode: Mode,
    pub nats_url: String,
    pub prometheus_bind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Production,
    Simulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub enabled: bool,
    pub speedup: f64,
    pub energy_variance: f64,
    pub failure_rate: f64,
    pub network_latency_ms: u64,
    pub random_seed: Option<u64>,
    pub contract_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfig {
    pub robot_id: Option<String>,
    pub klipper_url: Option<String>,
    pub moonraker_url: Option<String>,
    pub power_sensor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineryConfig {
    pub ingot_size: u32, // should be 3600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintConfig {
    pub certificate_ingots: u32, // should be 1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqConfig {
    pub orchestration: Orchestration,
    pub simulation: Simulation,
    pub robot: Option<RobotConfig>,
    pub refinery: Option<RefineryConfig>,
    pub mint: Option<MintConfig>,
}

impl RoboTorqConfig {
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).map_err(|e| e.to_string())
        } else {
            toml::from_str(&content).map_err(|e| e.to_string())
        }
    }
}
