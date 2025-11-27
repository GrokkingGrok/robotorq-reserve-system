use serde::{Deserialize, Serialize};
use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;
use crate::util::config::orchestration::Orchestration;
use crate::util::config::simulation::Simulation;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqConfig {
    #[serde(default = "default_robotorq_config_schema_version")]
    pub schema_version: u32,
    pub orchestration: Orchestration,
    pub simulation: Simulation,
    // Future optional components
    // pub robot: Option<RobotConfig>,
    // pub refinery: Option<RefineryConfig>,
    // pub mint: Option<MintConfig>,
}

fn default_robotorq_config_schema_version() -> u32 { ROBOTORQ_CONFIG_SCHEMA_VERSION }

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
