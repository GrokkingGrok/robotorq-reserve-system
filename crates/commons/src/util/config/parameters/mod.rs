use serde::Deserialize;
use tracing::{info, warn};

#[derive(Debug, Clone, Deserialize)]
pub struct PortsConfig {
    #[serde(rename = "ROBOT_GATEWAY_PORT", default = "default_robot_gateway_port")] 
    pub robot_gateway_port: u16,
    #[serde(rename = "METRICS_PORT", default = "default_metrics_port")] 
    pub metrics_port: u16,
    #[serde(rename = "GRAFANA_PORT", default = "default_grafana_port")] 
    pub grafana_port: u16,
}

fn default_robot_gateway_port() -> u16 { 9000 }
fn default_metrics_port() -> u16 { 8005 }
fn default_grafana_port() -> u16 { 8015 }

/// Load ports configuration from the repository default TOML file.
/// Fallback to defaults if the file is missing or invalid.
pub fn load_ports_config_from_default() -> PortsConfig {
    let rel = "crates/commons/src/util/config/parameters/ports.toml";
    let p = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let path = p.join(rel);
    match std::fs::read_to_string(&path) {
        Ok(s) => match toml::from_str::<PortsConfig>(&s) {
            Ok(cfg) => {
                info!(component = "commons.config", path = %path.display(), "loaded ports config");
                cfg
            }
            Err(e) => {
                warn!(component = "commons.config", path = %path.display(), error = %e, "failed to parse ports config; using defaults");
                PortsConfig { 
                    robot_gateway_port: default_robot_gateway_port(),
                    metrics_port: default_metrics_port(),
                    grafana_port: default_grafana_port(),
                }
            }
        },
        Err(_) => {
            // Silent fallback with a small info: file missing is acceptable
            info!(component = "commons.config", path = %path.display(), "ports config not found; using defaults");
            PortsConfig { 
                robot_gateway_port: default_robot_gateway_port(),
                metrics_port: default_metrics_port(),
                grafana_port: default_grafana_port(),
            }
        }
    }
}
