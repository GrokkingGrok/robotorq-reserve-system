#[cfg(test)]
mod tests {
    use crate::VaultConfig;

    #[cfg(feature = "simulation")]
    #[test]
    fn test_simulation_defaults() {
        // Test that simulation config has sensible defaults
        let config = VaultConfig::from_env();

        // Should default to production mode
        assert_eq!(config.simulation_mode, false);
        assert_eq!(config.time_compression_factor, 1.0);
        assert_eq!(config.drip_duration_hours, 24.0);
        assert_eq!(config.economic_variance_factor, 0.0);
        assert_eq!(config.simulation_scenario_id, None);
        // Single package delivery should be disabled by default
        assert_eq!(config.single_package_delivery, false);
        assert_eq!(config.package_signing_enabled, false);
        assert_eq!(config.package_hash_verification, false);
    }

    #[cfg(not(feature = "simulation"))]
    #[test]
    fn test_production_defaults() {
        // Test that production config has expected defaults
        let config = VaultConfig::from_env();

        // Single package delivery should be disabled by default
        assert_eq!(config.single_package_delivery, false);
        assert_eq!(config.package_signing_enabled, false);
        assert_eq!(config.package_hash_verification, false);
    }
}