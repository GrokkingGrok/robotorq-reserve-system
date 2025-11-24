#[cfg(feature = "simulation")]
#[cfg(test)]
mod simulation_tests {
    use super::*;

    #[test]
    fn test_simulation_config_defaults() {
        // Test that simulation config loads with defaults
        let config = MintConfig::from_env().unwrap();
        assert_eq!(config.simulation_mode, false);
        assert_eq!(config.time_compression_factor, 1.0);
        assert_eq!(config.ingot_generation_rate, 10.0);
        assert_eq!(config.batch_processing_delay_ms, 100);
        assert_eq!(config.proof_signing_delay_ms, 500);
        assert!(config.simulation_scenario_id.is_none());
    }

    #[test]
    fn test_simulation_config_from_env() {
        // Set environment variables
        std::env::set_var("SIMULATION_MODE", "true");
        std::env::set_var("SIMULATION_TIME_COMPRESSION", "1000.0");
        std::env::set_var("MINT_INGOT_GENERATION_RATE", "50.0");
        std::env::set_var("MINT_BATCH_PROCESSING_DELAY_MS", "10");
        std::env::set_var("MINT_PROOF_SIGNING_DELAY_MS", "50");
        std::env::set_var("SIMULATION_SCENARIO_ID", "test-scenario");

        let config = MintConfig::from_env().unwrap();
        assert_eq!(config.simulation_mode, true);
        assert_eq!(config.time_compression_factor, 1000.0);
        assert_eq!(config.ingot_generation_rate, 50.0);
        assert_eq!(config.batch_processing_delay_ms, 10);
        assert_eq!(config.proof_signing_delay_ms, 50);
        assert_eq!(config.simulation_scenario_id, Some("test-scenario".to_string()));

        // Clean up
        std::env::remove_var("SIMULATION_MODE");
        std::env::remove_var("SIMULATION_TIME_COMPRESSION");
        std::env::remove_var("MINT_INGOT_GENERATION_RATE");
        std::env::remove_var("MINT_BATCH_PROCESSING_DELAY_MS");
        std::env::remove_var("MINT_PROOF_SIGNING_DELAY_MS");
        std::env::remove_var("SIMULATION_SCENARIO_ID");
    }
}