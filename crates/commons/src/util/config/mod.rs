pub mod mode;
pub mod simulation;
pub mod ports;
// Re-export common config types for ergonomic imports
pub use mode::Mode;
pub use simulation::Simulation;
pub use ports::PortsConfig;
use serde::{Deserialize, Serialize};
use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;

use tracing::{info, warn};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboTorqConfig {
    #[serde(default = "default_robotorq_config_schema_version")]
    pub schema_version: u32,
    pub mode: Mode,
    pub simulation: Simulation,
    pub ports: ports::PortsConfig,
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
