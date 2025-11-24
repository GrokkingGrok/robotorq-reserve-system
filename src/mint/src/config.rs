use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintConfig {
    pub nats_url: String,
    pub ingots_per_cert: usize,
    // Batching configuration
    pub batch_threshold_count: usize, // Number of certificates per batch
    pub batch_threshold_seconds: i64, // Time window for batching (seconds)
    // Proof configuration
    pub proof_interval_count: usize, // Number of certificates per proof
    pub proof_interval_seconds: i64, // Time window for proof creation (seconds)
    pub merkle_tree_depth: usize, // Maximum merkle tree depth
    // Crypto configuration
    pub enable_crypto: bool, // Enable cryptographic signing
    pub signature_algorithm: String, // "dilithium5", "sphincs+", "none"
    pub min_stake_micro_rt: i64, // Minimum stake required (in micro-RT units)
    pub key_storage_path: Option<String>, // Path to store cryptographic keys
    // Archive configuration
    pub enable_archive: bool, // Enable in-memory archival of certificates & proofs
    // Simulation configuration (only available when simulation feature is enabled)
    #[cfg(feature = "simulation")]
    pub simulation_mode: bool,           // Enable simulation-specific behaviors
    #[cfg(feature = "simulation")]
    pub time_compression_factor: f64,    // Speed up time (1.0 = real-time, 1000.0 = 1000x faster)
    #[cfg(feature = "simulation")]
    pub ingot_generation_rate: f64,      // Rate of ingot arrival (ingots/second in simulation time)
    #[cfg(feature = "simulation")]
    pub batch_processing_delay_ms: i64,  // Artificial delay for batch processing
    #[cfg(feature = "simulation")]
    pub proof_signing_delay_ms: i64,     // Artificial delay for proof signing
    #[cfg(feature = "simulation")]
    pub simulation_scenario_id: Option<String>, // Tag for simulation runs
}

impl MintConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            nats_url: env::var("MINT_NATS_URL")
                .or_else(|_| env::var("NATS_URL"))
                .unwrap_or_else(|_| "nats://nats:4222".to_string()),
            ingots_per_cert: env::var("MINT_INGOTS_PER_CERT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            batch_threshold_count: env::var("MINT_BATCH_THRESHOLD_COUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10), // 10 certificates per batch
            batch_threshold_seconds: env::var("MINT_BATCH_THRESHOLD_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300), // 5 minutes
            proof_interval_count: env::var("MINT_PROOF_INTERVAL_COUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100), // 100 certificates per proof
            proof_interval_seconds: env::var("MINT_PROOF_INTERVAL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600), // 1 hour
            merkle_tree_depth: env::var("MINT_MERKLE_TREE_DEPTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(16), // 2^16 = 65536 leaves
            enable_crypto: env::var("MINT_ENABLE_CRYPTO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            signature_algorithm: env::var("MINT_SIGNATURE_ALGORITHM")
                .unwrap_or_else(|_| "dilithium5".to_string()),
            min_stake_micro_rt: env::var("MINT_MIN_STAKE_MICRO_RT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50000), // 0.05 RT minimum stake
            key_storage_path: env::var("MINT_KEY_STORAGE_PATH").ok(),
            enable_archive: env::var("MINT_ENABLE_ARCHIVE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            #[cfg(feature = "simulation")]
            simulation_mode: env::var("SIMULATION_MODE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            #[cfg(feature = "simulation")]
            time_compression_factor: env::var("SIMULATION_TIME_COMPRESSION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            #[cfg(feature = "simulation")]
            ingot_generation_rate: env::var("MINT_INGOT_GENERATION_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0), // 10 ingots/second
            #[cfg(feature = "simulation")]
            batch_processing_delay_ms: env::var("MINT_BATCH_PROCESSING_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100), // 100ms delay
            #[cfg(feature = "simulation")]
            proof_signing_delay_ms: env::var("MINT_PROOF_SIGNING_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500), // 500ms delay
            #[cfg(feature = "simulation")]
            simulation_scenario_id: env::var("SIMULATION_SCENARIO_ID").ok(),
        })
    }
}

impl Default for MintConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://nats:4222".to_string(),
            ingots_per_cert: 1000,
            batch_threshold_count: 10,
            batch_threshold_seconds: 300,
            proof_interval_count: 100,
            proof_interval_seconds: 3600,
            merkle_tree_depth: 16,
            enable_crypto: false,
            signature_algorithm: "dilithium5".to_string(),
            min_stake_micro_rt: 50000,
            key_storage_path: None,
            enable_archive: true,
            #[cfg(feature = "simulation")]
            simulation_mode: false,
            #[cfg(feature = "simulation")]
            time_compression_factor: 1.0,
            #[cfg(feature = "simulation")]
            ingot_generation_rate: 10.0,
            #[cfg(feature = "simulation")]
            batch_processing_delay_ms: 100,
            #[cfg(feature = "simulation")]
            proof_signing_delay_ms: 500,
            #[cfg(feature = "simulation")]
            simulation_scenario_id: None,
        }
    }
}