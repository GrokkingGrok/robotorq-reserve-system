use std::env;

#[derive(Clone, Debug)]
pub struct VaultConfig {
    pub nats_url: String,
    pub distostream_default_seconds: i64, // retained for future use
    pub distostream_tick_millis: i64,      // retained for future use
    pub db_url: String,
    // Contract approval configuration
    pub approval_enabled: bool,
    pub min_available_stake_ratio: f64, // Minimum stake ratio before approving (0.0-1.0)
    pub max_concurrent_contracts: usize, // Max contracts to approve simultaneously
    // Single package delivery configuration
    pub single_package_delivery: bool,   // Enable single package delivery instead of drips
    pub package_signing_enabled: bool,   // Enable cryptographic signing of packages
    pub package_hash_verification: bool, // Enable hash-based confirmations
    // Simulation configuration (only available when simulation feature is enabled)
    #[cfg(feature = "simulation")]
    pub simulation_mode: bool,           // Enable simulation-specific behaviors
    #[cfg(feature = "simulation")]
    pub time_compression_factor: f64,    // Speed up time (1.0 = real-time, 1000.0 = 1000x faster)
    #[cfg(feature = "simulation")]
    pub drip_processing_interval_seconds: i64, // How often to process drips (in simulation time)
    #[cfg(feature = "simulation")]
    pub drip_algorithm: String,          // Drip release algorithm: "uniform", "exponential", "linear"
    #[cfg(feature = "simulation")]
    pub drip_duration_hours: f64,        // How long drips last (default: 24.0)
    #[cfg(feature = "simulation")]
    pub drip_algorithm_param: f64,       // Algorithm-specific parameter (e.g., decay rate)
    #[cfg(feature = "simulation")]
    pub economic_variance_factor: f64,   // Random variance in economic calculations (± factor)
    #[cfg(feature = "simulation")]
    pub simulation_scenario_id: Option<String>, // Tag for simulation runs
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
        let db_url = env::var("VAULT_DB_URL")
            .unwrap_or_else(|_| "host=postgres user=torq password=torqpass dbname=roboTorq".to_string());
        let approval_enabled = env::var("VAULT_APPROVAL_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false); // Disabled by default for MVP
        let min_available_stake_ratio = env::var("VAULT_MIN_STAKE_RATIO")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1); // Require 10% of total stake available
        let max_concurrent_contracts = env::var("VAULT_MAX_CONCURRENT_CONTRACTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10); // Allow up to 10 concurrent contracts
        let single_package_delivery = env::var("VAULT_SINGLE_PACKAGE_DELIVERY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false); // Disabled by default, keep drips as fallback
        let package_signing_enabled = env::var("VAULT_PACKAGE_SIGNING_ENABLED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false); // Disabled by default for MVP
        let package_hash_verification = env::var("VAULT_PACKAGE_HASH_VERIFICATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false); // Disabled by default for MVP
        #[cfg(feature = "simulation")]
        let simulation_mode = env::var("SIMULATION_MODE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);
        #[cfg(feature = "simulation")]
        let time_compression_factor = env::var("SIMULATION_TIME_COMPRESSION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        #[cfg(feature = "simulation")]
        let drip_processing_interval_seconds = env::var("VAULT_DRIP_PROCESSING_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300); // 5 minutes by default
        #[cfg(feature = "simulation")]
        let drip_algorithm = env::var("VAULT_DRIP_ALGORITHM")
            .unwrap_or_else(|_| "uniform".to_string());
        #[cfg(feature = "simulation")]
        let drip_duration_hours = env::var("VAULT_DRIP_DURATION_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24.0); // 24 hours by default
        #[cfg(feature = "simulation")]
        let drip_algorithm_param = env::var("VAULT_DRIP_ALGORITHM_PARAM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0); // Default parameter (24 * 60 * 60)
        #[cfg(feature = "simulation")]
        let economic_variance_factor = env::var("SIMULATION_ECONOMIC_VARIANCE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0); // No variance by default
        #[cfg(feature = "simulation")]
        let simulation_scenario_id = env::var("SIMULATION_SCENARIO_ID").ok();
        Self {
            nats_url,
            distostream_default_seconds,
            distostream_tick_millis,
            db_url,
            approval_enabled,
            min_available_stake_ratio,
            max_concurrent_contracts,
            single_package_delivery,
            package_signing_enabled,
            package_hash_verification,
            #[cfg(feature = "simulation")]
            simulation_mode,
            #[cfg(feature = "simulation")]
            time_compression_factor,
            #[cfg(feature = "simulation")]
            drip_processing_interval_seconds,
            #[cfg(feature = "simulation")]
            drip_algorithm,
            #[cfg(feature = "simulation")]
            drip_duration_hours,
            #[cfg(feature = "simulation")]
            drip_algorithm_param,
            #[cfg(feature = "simulation")]
            economic_variance_factor,
            #[cfg(feature = "simulation")]
            simulation_scenario_id,
        }
    }
}
