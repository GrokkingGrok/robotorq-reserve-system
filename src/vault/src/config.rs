use std::env;

#[derive(Clone, Debug)]
pub struct VaultConfig {
    pub nats_url: String,
    pub distostream_default_seconds: i64, // retained for future use
    pub distostream_tick_millis: i64,      // retained for future use
}

impl VaultConfig {
    pub fn from_env() -> Self {
        // Prefer explicit vault-specific variable, then generic NATS_URL, then container-network default service name.
        let nats_url = env::var("VAULT_NATS_URL")
            .or_else(|_| env::var("NATS_URL"))
            .unwrap_or_else(|_| "nats://nats:4222".to_string());
        let distostream_default_seconds = env::var("VAULT_DISTOSTREAM_DEFAULT_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        let distostream_tick_millis = env::var("VAULT_DISTOSTREAM_TICK_MILLIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);
        Self { nats_url, distostream_default_seconds, distostream_tick_millis }
    }
}
