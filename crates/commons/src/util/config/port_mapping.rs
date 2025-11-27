//! Centralized port mappings for services in the RoboTorq stack.
//! Env-backed getters with sensible defaults for local dev.

fn parse_port(var: &str, default: u16) -> u16 {
	std::env::var(var)
		.ok()
		.and_then(|s| s.parse::<u16>().ok())
		.unwrap_or(default)
}

/// Metrics HTTP port (local dev default 8075).
pub fn metrics_port() -> u16 { parse_port("METRICS_PORT", 8075) }

/// Grafana HTTP port (local dev default 8085).
pub fn grafana_port() -> u16 { parse_port("GRAFANA_PORT", 8085) }

/// HTTP port for the `robot-gateway` service (local dev default 9000).
pub fn robot_gateway_port() -> u16 { parse_port("ROBOT_GATEWAY_PORT", 9000) }

/* Future services:
pub fn aggregator_service_port() -> u16 { parse_port("AGGREGATOR_SERVICE_PORT", 9010) }
pub fn refinery_service_port() -> u16 { parse_port("REFINERY_SERVICE_PORT", 9020) }
pub fn mint_service_port() -> u16 { parse_port("MINT_SERVICE_PORT", 9030) }
pub fn vault_service_port() -> u16 { parse_port("VAULT_SERVICE_PORT", 9040) }
pub fn wallet_service_port() -> u16 { parse_port("WALLET_SERVICE_PORT", 9050) }
*/

