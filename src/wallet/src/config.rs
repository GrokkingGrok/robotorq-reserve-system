//! Configuration management for the wallet service
//!
//! Loads configuration from environment variables with validation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Wallet service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP server port
    pub http_port: u16,
    /// NATS server URL
    pub nats_url: String,
    /// PostgreSQL database URL
    pub database_url: String,
    /// Optional wallet ID for persistence
    pub wallet_id: Option<String>,
    /// Mint verification endpoint
    pub mint_verification_url: String,
    /// Log level
    pub log_level: String,
    /// Enable metrics
    pub metrics_enabled: bool,
    /// Metrics port
    pub metrics_port: u16,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            http_port: 8080,
            nats_url: "nats://nats:4222".to_string(),
            database_url: "postgres://wallet:wallet_pass@postgres:5432/wallet_db".to_string(),
            wallet_id: None,
            mint_verification_url: "http://mint:8080/verify/certificate".to_string(),
            log_level: "info".to_string(),
            metrics_enabled: true,
            metrics_port: 9094,
            #[cfg(feature = "simulation")]
            simulation_mode: false,
            #[cfg(feature = "simulation")]
            time_compression_factor: 1.0,
            #[cfg(feature = "simulation")]
            drip_processing_interval_seconds: 300, // 5 minutes by default
            #[cfg(feature = "simulation")]
            drip_algorithm: "uniform".to_string(),
            #[cfg(feature = "simulation")]
            drip_duration_hours: 24.0, // 24 hours by default
            #[cfg(feature = "simulation")]
            drip_algorithm_param: 1.0, // Default parameter
            #[cfg(feature = "simulation")]
            economic_variance_factor: 0.0, // No variance by default
            #[cfg(feature = "simulation")]
            simulation_scenario_id: None,
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(port) = std::env::var("HTTP_PORT") {
            config.http_port = port.parse().context("Invalid HTTP_PORT")?;
        }

        if let Ok(url) = std::env::var("NATS_URL") {
            config.nats_url = url;
        }

        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database_url = url;
        }

        if let Ok(id) = std::env::var("WALLET_ID") {
            config.wallet_id = Some(id);
        }

        if let Ok(url) = std::env::var("MINT_VERIFICATION_URL") {
            config.mint_verification_url = url;
        }

        if let Ok(level) = std::env::var("RUST_LOG") {
            config.log_level = level;
        }

        if let Ok(enabled) = std::env::var("METRICS_ENABLED") {
            config.metrics_enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(port) = std::env::var("METRICS_PORT") {
            config.metrics_port = port.parse().context("Invalid METRICS_PORT")?;
        }

        // Simulation configuration (only when simulation feature is enabled)
        #[cfg(feature = "simulation")]
        {
            if let Ok(mode) = std::env::var("SIMULATION_MODE") {
                config.simulation_mode = mode.parse().unwrap_or(false);
            }

            if let Ok(factor) = std::env::var("SIMULATION_TIME_COMPRESSION") {
                config.time_compression_factor = factor.parse().unwrap_or(1.0);
            }

            if let Ok(interval) = std::env::var("WALLET_DRIP_PROCESSING_INTERVAL_SECONDS") {
                config.drip_processing_interval_seconds = interval.parse().unwrap_or(300);
            }

            if let Ok(algorithm) = std::env::var("WALLET_DRIP_ALGORITHM") {
                config.drip_algorithm = algorithm;
            }

            if let Ok(duration) = std::env::var("WALLET_DRIP_DURATION_HOURS") {
                config.drip_duration_hours = duration.parse().unwrap_or(24.0);
            }

            if let Ok(param) = std::env::var("WALLET_DRIP_ALGORITHM_PARAM") {
                config.drip_algorithm_param = param.parse().unwrap_or(1.0);
            }

            if let Ok(variance) = std::env::var("SIMULATION_ECONOMIC_VARIANCE") {
                config.economic_variance_factor = variance.parse().unwrap_or(0.0);
            }

            config.simulation_scenario_id = std::env::var("SIMULATION_SCENARIO_ID").ok();
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.http_port == 0 {
            anyhow::bail!("HTTP port cannot be 0");
        }

        if self.nats_url.is_empty() {
            anyhow::bail!("NATS URL cannot be empty");
        }

        if self.database_url.is_empty() {
            anyhow::bail!("Database URL cannot be empty");
        }

        if self.metrics_enabled && self.metrics_port == 0 {
            anyhow::bail!("Metrics port cannot be 0 when metrics are enabled");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_load_from_env() {
        // Set environment variables
        unsafe {
            std::env::set_var("HTTP_PORT", "9090");
            std::env::set_var("NATS_URL", "nats://test:4222");
            std::env::set_var("DATABASE_URL", "postgres://test:test@localhost/test");
            std::env::set_var("WALLET_ID", "test-wallet-001");
            std::env::set_var("MINT_VERIFICATION_URL", "http://test-mint:8080/verify");
            std::env::set_var("RUST_LOG", "debug");
            std::env::set_var("METRICS_ENABLED", "false");
            std::env::set_var("METRICS_PORT", "9091");

            // Simulation variables (only when feature is enabled)
            #[cfg(feature = "simulation")]
            {
                std::env::set_var("SIMULATION_MODE", "true");
                std::env::set_var("SIMULATION_TIME_COMPRESSION", "10.0");
                std::env::set_var("WALLET_DRIP_PROCESSING_INTERVAL_SECONDS", "600");
                std::env::set_var("WALLET_DRIP_ALGORITHM", "exponential");
                std::env::set_var("WALLET_DRIP_DURATION_HOURS", "48.0");
                std::env::set_var("WALLET_DRIP_ALGORITHM_PARAM", "0.5");
                std::env::set_var("SIMULATION_ECONOMIC_VARIANCE", "0.1");
                std::env::set_var("SIMULATION_SCENARIO_ID", "test-scenario-001");
            }
        }

        let config = Config::load().unwrap();

        assert_eq!(config.http_port, 9090);
        assert_eq!(config.nats_url, "nats://test:4222");
        assert_eq!(config.database_url, "postgres://test:test@localhost/test");
        assert_eq!(config.wallet_id, Some("test-wallet-001".to_string()));
        assert_eq!(config.mint_verification_url, "http://test-mint:8080/verify");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.metrics_enabled, false);
        assert_eq!(config.metrics_port, 9091);

        // Test simulation config (only when feature is enabled)
        #[cfg(feature = "simulation")]
        {
            assert_eq!(config.simulation_mode, true);
            assert_eq!(config.time_compression_factor, 10.0);
            assert_eq!(config.drip_processing_interval_seconds, 600);
            assert_eq!(config.drip_algorithm, "exponential");
            assert_eq!(config.drip_duration_hours, 48.0);
            assert_eq!(config.drip_algorithm_param, 0.5);
            assert_eq!(config.economic_variance_factor, 0.1);
            assert_eq!(config.simulation_scenario_id, Some("test-scenario-001".to_string()));
        }

        // Clean up environment
        unsafe {
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("NATS_URL");
            std::env::remove_var("DATABASE_URL");
            std::env::remove_var("WALLET_ID");
            std::env::remove_var("MINT_VERIFICATION_URL");
            std::env::remove_var("RUST_LOG");
            std::env::remove_var("METRICS_ENABLED");
            std::env::remove_var("METRICS_PORT");

            // Clean up simulation variables
            #[cfg(feature = "simulation")]
            {
                std::env::remove_var("SIMULATION_MODE");
                std::env::remove_var("SIMULATION_TIME_COMPRESSION");
                std::env::remove_var("WALLET_DRIP_PROCESSING_INTERVAL_SECONDS");
                std::env::remove_var("WALLET_DRIP_ALGORITHM");
                std::env::remove_var("WALLET_DRIP_DURATION_HOURS");
                std::env::remove_var("WALLET_DRIP_ALGORITHM_PARAM");
                std::env::remove_var("SIMULATION_ECONOMIC_VARIANCE");
                std::env::remove_var("SIMULATION_SCENARIO_ID");
            }
        }
    }

    #[test]
    #[serial]
    fn test_config_load_partial_env() {
        // Clean up any existing environment variables first
        unsafe {
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("NATS_URL");
        }

        // Set only some environment variables
        unsafe {
            std::env::set_var("HTTP_PORT", "8081");
            std::env::set_var("NATS_URL", "nats://partial:4222");
        }

        let config = Config::load().unwrap();

        assert_eq!(config.http_port, 8081);
        assert_eq!(config.nats_url, "nats://partial:4222");
        // Other fields should use defaults
        assert_eq!(config.database_url, "postgres://wallet:wallet_pass@postgres:5432/wallet_db");
        assert_eq!(config.wallet_id, None);

        // Clean up
        unsafe {
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("NATS_URL");
        }
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let mut config = Config::default();
        config.http_port = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_urls() {
        let mut config = Config::default();
        config.nats_url = "".to_string();
        assert!(config.validate().is_err());

        let mut config2 = Config::default();
        config2.database_url = "".to_string();
        assert!(config2.validate().is_err());
    }

    #[test]
    fn test_config_validation_metrics_port_zero() {
        let mut config = Config::default();
        config.metrics_enabled = true;
        config.metrics_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_metrics_port_zero_disabled() {
        let mut config = Config::default();
        config.metrics_enabled = false;
        config.metrics_port = 0;
        // Should be ok when metrics are disabled
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_simulation_config_defaults() {
        let config = Config::default();

        // Test simulation defaults (only when feature is enabled)
        #[cfg(feature = "simulation")]
        {
            assert_eq!(config.simulation_mode, false);
            assert_eq!(config.time_compression_factor, 1.0);
            assert_eq!(config.drip_processing_interval_seconds, 300);
            assert_eq!(config.drip_algorithm, "uniform");
            assert_eq!(config.drip_duration_hours, 24.0);
            assert_eq!(config.drip_algorithm_param, 1.0);
            assert_eq!(config.economic_variance_factor, 0.0);
            assert_eq!(config.simulation_scenario_id, None);
        }
    }
}