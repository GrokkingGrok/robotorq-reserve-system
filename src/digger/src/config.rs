// Configuration - Environment variables and defaults
// Phase 1: Digger Rewrite - Day 3

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Digger Configuration
/// 
/// Loaded from environment variables with sensible defaults.
/// Can be customized per deployment (dev, testnet, mainnet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiggerConfig {
    /// Unique identifier for this Digger instance
    /// Example: "digger-dev-001", "digger-prod-alice-miner-1"
    pub digger_id: String,
    
    /// NATS server URL for message passing
    /// Default: "nats://localhost:4222"
    pub nats_url: String,
    
    /// HTTP server port for API endpoints
    /// Default: 9000
    pub http_port: u16,
    
    /// Local storage path for JTU databases
    /// Default: "./jtu_storage"
    pub storage_path: PathBuf,
    
    /// How often to send hash batches to Refinery (seconds)
    /// Default: 60 (1 minute)
    /// Production: Could be 300 (5 minutes) to reduce bandwidth
    pub batch_interval_sec: u64,
    
    /// How long to keep JTUs locally before pruning (days)
    /// Default: 30 (Digger's responsibility period)
    pub prune_after_days: i64,
    
    /// Log level (trace, debug, info, warn, error)
    /// Default: "info"
    pub log_level: String,
}

impl DiggerConfig {
    /// Load configuration from environment variables
    /// 
    /// Environment variables:
    /// - DIGGER_ID (required)
    /// - NATS_URL (default: nats://localhost:4222)
    /// - HTTP_PORT (default: 9000)
    /// - STORAGE_PATH (default: ./jtu_storage)
    /// - BATCH_INTERVAL_SEC (default: 60)
    /// - PRUNE_AFTER_DAYS (default: 30)
    /// - LOG_LEVEL (default: info)
    pub fn from_env() -> Result<Self, String> {
        // Load .env file if it exists (for local development)
        dotenvy::dotenv().ok();
        
        let digger_id = std::env::var("DIGGER_ID")
            .map_err(|_| "DIGGER_ID environment variable is required".to_string())?;
        
        let nats_url = std::env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        
        let http_port = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid HTTP_PORT: {}", e))?;
        
        let storage_path = std::env::var("STORAGE_PATH")
            .unwrap_or_else(|_| "./jtu_storage".to_string())
            .into();
        
        let batch_interval_sec = std::env::var("BATCH_INTERVAL_SEC")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .map_err(|e| format!("Invalid BATCH_INTERVAL_SEC: {}", e))?;
        
        let prune_after_days = std::env::var("PRUNE_AFTER_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<i64>()
            .map_err(|e| format!("Invalid PRUNE_AFTER_DAYS: {}", e))?;
        
        let log_level = std::env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());
        
        Ok(Self {
            digger_id,
            nats_url,
            http_port,
            storage_path,
            batch_interval_sec,
            prune_after_days,
            log_level,
        })
    }
    
    /// Create default configuration for testing
    pub fn default_test() -> Self {
        Self {
            digger_id: "test-digger".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            http_port: 9000,
            storage_path: "./jtu_storage".into(),
            batch_interval_sec: 60,
            prune_after_days: 30,
            log_level: "debug".to_string(),
        }
    }
    
    /// Create configuration for development
    pub fn default_dev() -> Self {
        Self {
            digger_id: "digger-dev-001".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            http_port: 9000,
            storage_path: "./jtu_storage".into(),
            batch_interval_sec: 60,  // 1 minute for fast iteration
            prune_after_days: 7,     // 7 days for dev (save disk space)
            log_level: "debug".to_string(),
        }
    }
    
    /// Create configuration for production
    pub fn default_prod() -> Self {
        Self {
            digger_id: "digger-prod-CHANGEME".to_string(), // Must override!
            nats_url: "nats://nats-server:4222".to_string(),
            http_port: 9000,
            storage_path: "/var/lib/digger/storage".into(),
            batch_interval_sec: 300,  // 5 minutes (reduce bandwidth)
            prune_after_days: 30,     // Full 30-day responsibility
            log_level: "info".to_string(),
        }
    }
}

impl Default for DiggerConfig {
    fn default() -> Self {
        Self::default_dev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_test_config() {
        let config = DiggerConfig::default_test();
        assert_eq!(config.digger_id, "test-digger");
        assert_eq!(config.http_port, 9000);
        assert_eq!(config.batch_interval_sec, 60);
    }

    #[test]
    fn test_default_dev_config() {
        let config = DiggerConfig::default_dev();
        assert_eq!(config.digger_id, "digger-dev-001");
        assert_eq!(config.prune_after_days, 7); // Shorter for dev
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_default_prod_config() {
        let config = DiggerConfig::default_prod();
        assert_eq!(config.batch_interval_sec, 300); // Longer for prod
        assert_eq!(config.prune_after_days, 30);    // Full period
        assert_eq!(config.log_level, "info");       // Less verbose
    }

    #[test]
    fn test_from_env_missing_digger_id() {
        // Clear env to test error case
        unsafe { std::env::remove_var("DIGGER_ID"); }
        
        let result = DiggerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("DIGGER_ID"));
    }

    #[test]
    fn test_from_env_with_all_vars() {
        // Set all env vars
        unsafe {
            std::env::set_var("DIGGER_ID", "test-env-digger");
            std::env::set_var("NATS_URL", "nats://custom:4222");
            std::env::set_var("HTTP_PORT", "8888");
            std::env::set_var("STORAGE_PATH", "/custom/path");
            std::env::set_var("BATCH_INTERVAL_SEC", "120");
            std::env::set_var("PRUNE_AFTER_DAYS", "14");
            std::env::set_var("LOG_LEVEL", "trace");
        }
        
        let config = DiggerConfig::from_env().unwrap();
        
        assert_eq!(config.digger_id, "test-env-digger");
        assert_eq!(config.nats_url, "nats://custom:4222");
        assert_eq!(config.http_port, 8888);
        assert_eq!(config.storage_path, PathBuf::from("/custom/path"));
        assert_eq!(config.batch_interval_sec, 120);
        assert_eq!(config.prune_after_days, 14);
        assert_eq!(config.log_level, "trace");
        
        // Cleanup
        unsafe {
            std::env::remove_var("DIGGER_ID");
            std::env::remove_var("NATS_URL");
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("STORAGE_PATH");
            std::env::remove_var("BATCH_INTERVAL_SEC");
            std::env::remove_var("PRUNE_AFTER_DAYS");
            std::env::remove_var("LOG_LEVEL");
        }
    }

    #[test]
    fn test_from_env_with_defaults() {
        // Clean up any lingering env vars from other tests
        unsafe {
            std::env::remove_var("DIGGER_ID");
            std::env::remove_var("NATS_URL");
            std::env::remove_var("HTTP_PORT");
            std::env::remove_var("STORAGE_PATH");
            std::env::remove_var("BATCH_INTERVAL_SEC");
            std::env::remove_var("PRUNE_AFTER_DAYS");
            std::env::remove_var("LOG_LEVEL");
        }
        
        // Only set required DIGGER_ID
        unsafe { std::env::set_var("DIGGER_ID", "minimal-digger"); }
        
        let config = DiggerConfig::from_env().unwrap();
        
        assert_eq!(config.digger_id, "minimal-digger");
        assert_eq!(config.nats_url, "nats://localhost:4222"); // Default
        assert_eq!(config.http_port, 9000);                   // Default
        assert_eq!(config.batch_interval_sec, 60);            // Default
        
        // Cleanup
        unsafe { std::env::remove_var("DIGGER_ID"); }
    }
}
