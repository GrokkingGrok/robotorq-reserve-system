pub mod orchestration;
pub mod mode;
pub mod simulation;
pub mod port_mapping;
use serde::{Deserialize, Serialize};
use crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION;
use crate::util::config::orchestration::Orchestration;
use crate::util::config::simulation::Simulation;
use dotenvy::from_path;
use tracing::{info, warn};



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

/// Load environment variables from a specific file if provided.
/// Set `ROBOTORQ_ENV_FILE` to a path (absolute or relative) to enable.
/// Returns Ok(()) even if the variable is unset or file missing; logs are caller's responsibility.
pub fn load_env_from_configurable_file() -> Result<(), String> {
    if let Ok(raw) = std::env::var("ROBOTORQ_ENV_FILE") {
        let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
        let mut p = std::path::PathBuf::from(trimmed);
        if p.is_relative() {
            if let Ok(cwd) = std::env::current_dir() { p = cwd.join(p); }
        }
        from_path(&p).map_err(|e| format!("failed to load env from {}: {}", p.display(), e))?;
        info!(component = "commons.config", path = %p.display(), "loaded env from ROBOTORQ_ENV_FILE");
        return Ok(());
    }

    // Fallbacks: try common filenames relative to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            ".env",
            "workspace.env",
            "crates/commons/src/util/config/.env",
        ];
        for rel in candidates {
            let p = cwd.join(rel);
            if p.exists() {
                match from_path(&p) {
                    Ok(_) => {
                        info!(component = "commons.config", path = %p.display(), "loaded env from fallback file");
                        return Ok(());
                    }
                    Err(e) => {
                        warn!(component = "commons.config", path = %p.display(), error = %e, "failed loading fallback env file");
                    }
                }
            }
        }
    }

    Ok(())
}
