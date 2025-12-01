//! Economic Configuration for RoboTorq Reserve System
//!
//! This module provides configuration for the economic parameters and monetary policy
//! of the RoboTorq Reserve System. It defines the economic invariants, reserve ratios,
//! token economics, and circulation parameters that govern the system's behavior.
//!
//! # Economic Invariants
//!
//! The system maintains strict economic invariants:
//! - **1 TokenTorqIngot = 3,600 JouleTorqOre units**
//! - **1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units**
//!
//! These invariants ensure the monetary system remains backed by measurable robotic labor.
//!
//! # Unit Hierarchy
//!
//! | Layer | Artifact | Aggregation | Resulting Count | Role |
//! |-------|----------|-------------|-----------------|------|
//! | L0 | JouleTorqOre | 1 token × 1 joule | 3,600 per ingot | Atomic work proof |
//! | L1 | TokenTorqIngot | 3,600 units of Ore | 1,000 per certificate | Batched for minting |
//! | L2 | RoboTorq Certificate | 1,000 ingots (3.6M units of Ore) | Basis for reserve | Monetary Backing |
//! | L3 | RoboTorqUnits | Certificate-Backed, Digital, 1:1 | Dynamic | Circulation |
//! | L4 | Bearer Bond | Certificate-Backed, Physical, N:1 mapping | Dynamic | Circulation |
//!
//! # Monetary Policy
//!
//! The system implements several monetary policy mechanisms:
//! - **Demurrage**: Continuous decay to discourage hoarding and encourage circulation
//! - **UBD (Universal Basic Dividend)**: Periodic redistribution to active participants
//! - **Stake Requirements**: Minimum stake for labor contracts to prevent inflation
//! - **Reserve Requirements**: Minimum backing ratios for certificates
//!
//! # Reserve Management
//!
//! The reserve system maintains multiple vault subsystems:
//! - **CertVault**: Holds certificate proofs and timing metadata
//! - **StakeVault**: Provides stake constraint for labor contracts
//! - **DistoVault**: Executes UBD payments and balance adjustments
//! - **ShortVault**: Liquid member balances for active circulation

use serde::{Deserialize, Serialize};

/// Economic configuration for the RoboTorq Reserve System.
///
/// Defines the economic parameters, monetary policy, and reserve management
/// settings that govern the behavior of the token economy.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::economic::{EconomicConfig, DemurrageModel};
///
/// // Production economic configuration
/// let prod_economic = EconomicConfig {
///     joule_per_ingot: 3600,  // Economic invariant
///     ingots_per_certificate: 1000,  // Economic invariant
///     demurrage_model: DemurrageModel::Continuous {
///         annual_rate: 0.02,  // 2% annual demurrage
///     },
///     ubd_enabled: true,
///     ubd_interval_hours: 24,  // Daily UBD
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    /// JouleTorqOre units per TokenTorqIngot (economic invariant).
    ///
    /// This fundamental invariant ensures each ingot represents exactly
    /// 3,600 units of atomic robotic work. Must not be changed in production.
    #[serde(default = "default_joule_per_ingot")]
    pub joule_per_ingot: u32,

    /// TokenTorqIngots per RoboTorq Certificate (economic invariant).
    ///
    /// This invariant ensures each certificate represents 1,000 ingots
    /// (3.6 million JouleTorqOre units). Must not be changed in production.
    #[serde(default = "default_ingots_per_certificate")]
    pub ingots_per_certificate: u32,

    /// Demurrage model configuration.
    ///
    /// Defines how idle RoboTorq is collected to encourage circulation
    /// and prevent hoarding.
    #[serde(default)]
    pub demurrage_model: DemurrageModel,

    /// Universal Basic Dividend (UBD) configuration.
    #[serde(default)]
    pub ubd: UbdConfig,

    /// Stake requirements for labor contracts.
    ///
    /// Minimum stake required to initiate robotic labor contracts,
    /// preventing uncontrolled inflation.
    #[serde(default)]
    pub stake: StakeConfig,

    /// Reserve management configuration.
    ///
    /// Settings for reserve ratios, vault management, and backing requirements.
    #[serde(default)]
    pub reserve: ReserveConfig,

    /// Token supply controls.
    ///
    /// Maximum supply limits and issuance controls for different token types.
    #[serde(default)]
    pub supply: SupplyConfig,

    /// Distribution and circulation parameters.
    ///
    /// Settings for token distribution, minimum spend thresholds,
    /// and circulation incentives.
    #[serde(default)]
    pub distribution: DistributionConfig,
}

impl Default for EconomicConfig {
    fn default() -> Self {
        Self {
            joule_per_ingot: default_joule_per_ingot(),
            ingots_per_certificate: default_ingots_per_certificate(),
            demurrage_model: DemurrageModel::default(),
            ubd: UbdConfig::default(),
            stake: StakeConfig::default(),
            reserve: ReserveConfig::default(),
            supply: SupplyConfig::default(),
            distribution: DistributionConfig::default(),
        }
    }
}

/// Demurrage model configurations.
///
/// Defines how token value decays over time to encourage economic circulation.
/// Demurrage prevents hoarding by continuously reducing token value unless actively used.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemurrageModel {
    /// No demurrage - tokens maintain constant value
    None,

    /// Continuous exponential decay
    Continuous {
        /// Annual demurrage rate (e.g., 0.02 = 2% per year)
        annual_rate: f64,
    },

    /// Step-wise demurrage with periodic resets
    Stepped {
        /// Demurrage rate per period
        rate_per_period: f64,

        /// Period length in hours
        period_hours: u32,

        /// Reset threshold (minimum balance to avoid reset)
        reset_threshold: u64,
    },
}

impl Default for DemurrageModel {
    fn default() -> Self {
        // Default to 2% annual continuous demurrage
        DemurrageModel::Continuous { annual_rate: 0.02 }
    }
}

/// Universal Basic Dividend (UBD) configuration.
///
/// UBD provides periodic redistribution of demurrage-collected funds
/// to active participants who meet minimum circulation requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbdConfig {
    /// Whether UBD is enabled
    #[serde(default = "default_ubd_enabled")]
    pub enabled: bool,

    /// UBD distribution interval in hours
    #[serde(default = "default_ubd_interval_hours")]
    pub interval_hours: u32,

    /// Minimum spend threshold for UBD eligibility
    ///
    /// Participants must spend at least this percentage of their holdings
    /// within the eligibility period to receive UBD.
    #[serde(default = "default_ubd_min_spend_threshold")]
    pub min_spend_threshold: f64,

    /// UBD eligibility period in hours
    ///
    /// Lookback period for determining UBD eligibility based on spending activity.
    #[serde(default = "default_ubd_eligibility_period_hours")]
    pub eligibility_period_hours: u32,
}

impl Default for UbdConfig {
    fn default() -> Self {
        Self {
            enabled: default_ubd_enabled(),
            interval_hours: default_ubd_interval_hours(),
            min_spend_threshold: default_ubd_min_spend_threshold(),
            eligibility_period_hours: default_ubd_eligibility_period_hours(),
        }
    }
}

/// Stake requirements for labor contracts.
///
/// Defines the minimum stake required to initiate robotic labor contracts,
/// ensuring economic accountability and preventing uncontrolled issuance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeConfig {
    /// Minimum stake as percentage of contract value
    #[serde(default = "default_stake_min_percentage")]
    pub min_percentage: f64,

    /// Maximum stake as percentage of contract value
    #[serde(default = "default_stake_max_percentage")]
    pub max_percentage: f64,

    /// Stake lockup period in hours
    #[serde(default = "default_stake_lockup_hours")]
    pub lockup_hours: u32,

    /// Early unstake penalty rate
    #[serde(default = "default_stake_penalty_rate")]
    pub penalty_rate: f64,
}

impl Default for StakeConfig {
    fn default() -> Self {
        Self {
            min_percentage: default_stake_min_percentage(),
            max_percentage: default_stake_max_percentage(),
            lockup_hours: default_stake_lockup_hours(),
            penalty_rate: default_stake_penalty_rate(),
        }
    }
}

/// Reserve management configuration.
///
/// Defines reserve ratios, vault management, and backing requirements
/// to ensure the monetary system remains solvent and backed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveConfig {
    /// Minimum reserve ratio for certificates
    ///
    /// Certificates must maintain at least this ratio of backing
    /// to total circulation (e.g., 1.0 = 100% backing).
    #[serde(default = "default_reserve_min_ratio")]
    pub min_ratio: f64,

    /// Target reserve ratio for optimal backing
    #[serde(default = "default_reserve_target_ratio")]
    pub target_ratio: f64,

    /// Reserve audit interval in hours
    #[serde(default = "default_reserve_audit_interval_hours")]
    pub audit_interval_hours: u32,

    /// Emergency reserve threshold
    ///
    /// If reserve ratio falls below this threshold, emergency measures are triggered.
    #[serde(default = "default_reserve_emergency_threshold")]
    pub emergency_threshold: f64,
}

impl Default for ReserveConfig {
    fn default() -> Self {
        Self {
            min_ratio: default_reserve_min_ratio(),
            target_ratio: default_reserve_target_ratio(),
            audit_interval_hours: default_reserve_audit_interval_hours(),
            emergency_threshold: default_reserve_emergency_threshold(),
        }
    }
}

/// Token supply controls.
///
/// Defines maximum supply limits and issuance controls for different token types
/// to prevent inflation and ensure controlled monetary expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyConfig {
    /// Maximum total JouleTorqOre supply
    #[serde(default = "default_supply_max_joule")]
    pub max_joule: Option<u128>,

    /// Maximum total TokenTorqIngot supply
    #[serde(default = "default_supply_max_ingot")]
    pub max_ingot: Option<u128>,

    /// Maximum total RoboTorq Certificate supply
    #[serde(default = "default_supply_max_certificate")]
    pub max_certificate: Option<u128>,

    /// Daily issuance limit for new certificates
    #[serde(default = "default_supply_daily_certificate_limit")]
    pub daily_certificate_limit: Option<u64>,
}

impl Default for SupplyConfig {
    fn default() -> Self {
        Self {
            max_joule: default_supply_max_joule(),
            max_ingot: default_supply_max_ingot(),
            max_certificate: default_supply_max_certificate(),
            daily_certificate_limit: default_supply_daily_certificate_limit(),
        }
    }
}

/// Distribution and circulation parameters.
///
/// Defines parameters for token distribution, minimum spend thresholds,
/// and circulation incentives to ensure active economic participation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Minimum spend threshold for UBD eligibility
    ///
    /// Percentage of holdings that must be spent to qualify for UBD.
    #[serde(default = "default_distribution_min_spend_percent")]
    pub min_spend_percent: f64,

    /// Distribution activation path requirements
    #[serde(default)]
    pub activation: ActivationConfig,

    /// BidNet marketplace configuration
    #[serde(default)]
    pub bidnet: BidnetConfig,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            min_spend_percent: default_distribution_min_spend_percent(),
            activation: ActivationConfig::default(),
            bidnet: BidnetConfig::default(),
        }
    }
}

/// Distribution activation configuration.
///
/// Defines requirements for activating distribution eligibility,
/// particularly for physical bearer bonds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationConfig {
    /// Minimum internal spend threshold for UBD eligibility
    ///
    /// Physical bearer bonds must be spent internally at least this
    /// percentage before becoming UBD-eligible.
    #[serde(default = "default_activation_min_internal_spend")]
    pub min_internal_spend: f64,

    /// Activation grace period in hours
    ///
    /// Time allowed for initial activation after bond issuance.
    #[serde(default = "default_activation_grace_period_hours")]
    pub grace_period_hours: u32,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            min_internal_spend: default_activation_min_internal_spend(),
            grace_period_hours: default_activation_grace_period_hours(),
        }
    }
}

/// BidNet marketplace configuration.
///
/// Defines parameters for the decentralized labor exchange and marketplace
/// where RoboTorq can be converted to goods and services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidnetConfig {
    /// Whether BidNet marketplace is enabled
    #[serde(default = "default_bidnet_enabled")]
    pub enabled: bool,

    /// Maximum bid amount as percentage of available balance
    #[serde(default = "default_bidnet_max_bid_percent")]
    pub max_bid_percent: f64,

    /// Bid expiry time in hours
    #[serde(default = "default_bidnet_expiry_hours")]
    pub expiry_hours: u32,

    /// Minimum bid amount
    #[serde(default = "default_bidnet_min_bid")]
    pub min_bid: u64,
}

impl Default for BidnetConfig {
    fn default() -> Self {
        Self {
            enabled: default_bidnet_enabled(),
            max_bid_percent: default_bidnet_max_bid_percent(),
            expiry_hours: default_bidnet_expiry_hours(),
            min_bid: default_bidnet_min_bid(),
        }
    }
}

// Default value functions

/// Default JouleTorqOre units per TokenTorqIngot (economic invariant).
fn default_joule_per_ingot() -> u32 { 3600 }

/// Default TokenTorqIngots per RoboTorq Certificate (economic invariant).
fn default_ingots_per_certificate() -> u32 { 1000 }

/// Default UBD enabled state.
fn default_ubd_enabled() -> bool { true }

/// Default UBD interval in hours.
fn default_ubd_interval_hours() -> u32 { 24 } // Daily

/// Default UBD minimum spend threshold.
fn default_ubd_min_spend_threshold() -> f64 { 0.75 } // 75%

/// Default UBD eligibility period in hours.
fn default_ubd_eligibility_period_hours() -> u32 { 168 } // 1 week

/// Default minimum stake percentage.
fn default_stake_min_percentage() -> f64 { 0.1 } // 10%

/// Default maximum stake percentage.
fn default_stake_max_percentage() -> f64 { 0.5 } // 50%

/// Default stake lockup period in hours.
fn default_stake_lockup_hours() -> u32 { 24 } // 1 day

/// Default stake penalty rate for early unstaking.
fn default_stake_penalty_rate() -> f64 { 0.05 } // 5%

/// Default minimum reserve ratio.
fn default_reserve_min_ratio() -> f64 { 1.0 } // 100%

/// Default target reserve ratio.
fn default_reserve_target_ratio() -> f64 { 1.2 } // 120%

/// Default reserve audit interval in hours.
fn default_reserve_audit_interval_hours() -> u32 { 24 } // Daily

/// Default emergency reserve threshold.
fn default_reserve_emergency_threshold() -> f64 { 0.95 } // 95%

/// Default maximum JouleTorqOre supply (None = unlimited).
fn default_supply_max_joule() -> Option<u128> { None }

/// Default maximum TokenTorqIngot supply (None = unlimited).
fn default_supply_max_ingot() -> Option<u128> { None }

/// Default maximum RoboTorq Certificate supply (None = unlimited).
fn default_supply_max_certificate() -> Option<u128> { None }

/// Default daily certificate issuance limit (None = unlimited).
fn default_supply_daily_certificate_limit() -> Option<u64> { None }

/// Default minimum spend percentage for distribution.
fn default_distribution_min_spend_percent() -> f64 { 0.75 } // 75%

/// Default minimum internal spend for activation.
fn default_activation_min_internal_spend() -> f64 { 0.75 } // 75%

/// Default activation grace period in hours.
fn default_activation_grace_period_hours() -> u32 { 168 } // 1 week

/// Default BidNet enabled state.
fn default_bidnet_enabled() -> bool { true }

/// Default maximum bid percentage.
fn default_bidnet_max_bid_percent() -> f64 { 0.1 } // 10%

/// Default bid expiry time in hours.
fn default_bidnet_expiry_hours() -> u32 { 24 } // 1 day

/// Default minimum bid amount.
fn default_bidnet_min_bid() -> u64 { 1 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economic_config_default() {
        let config = EconomicConfig::default();
        assert_eq!(config.joule_per_ingot, 3600);
        assert_eq!(config.ingots_per_certificate, 1000);
        assert!(config.ubd.enabled);
        assert_eq!(config.ubd.interval_hours, 24);
    }

    #[test]
    fn test_economic_config_serialization() {
        let config = EconomicConfig {
            joule_per_ingot: 3600,
            ingots_per_certificate: 1000,
            demurrage_model: DemurrageModel::Continuous { annual_rate: 0.02 },
            ubd: UbdConfig {
                enabled: true,
                interval_hours: 24,
                min_spend_threshold: 0.75,
                eligibility_period_hours: 168,
            },
            ..Default::default()
        };

        // Test TOML serialization
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("joule_per_ingot = 3600"));
        assert!(toml.contains("ingots_per_certificate = 1000"));
        assert!(toml.contains("[demurrage_model.continuous]"));
        assert!(toml.contains("annual_rate = 0.02"));

        // Test deserialization
        let deserialized: EconomicConfig = toml::from_str(&toml).unwrap();
        assert_eq!(deserialized.joule_per_ingot, 3600);
        assert_eq!(deserialized.ingots_per_certificate, 1000);
        match deserialized.demurrage_model {
            DemurrageModel::Continuous { annual_rate } => assert_eq!(annual_rate, 0.02),
            _ => panic!("Expected continuous demurrage"),
        }
    }

    #[test]
    fn test_demurrage_model_variants() {
        // Test None variant
        let none = DemurrageModel::None;
        let config = EconomicConfig {
            demurrage_model: none,
            ..Default::default()
        };
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("demurrage_model = \"none\""));

        // Test Continuous variant
        let continuous = DemurrageModel::Continuous { annual_rate: 0.05 };
        let config = EconomicConfig {
            demurrage_model: continuous,
            ..Default::default()
        };
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("[demurrage_model.continuous]"));
        assert!(toml.contains("annual_rate = 0.05"));
    }

    #[test]
    fn test_reserve_config_validation() {
        let reserve = ReserveConfig {
            min_ratio: 1.0,
            target_ratio: 1.2,
            audit_interval_hours: 24,
            emergency_threshold: 0.95,
        };

        let config = EconomicConfig {
            reserve,
            ..Default::default()
        };

        let toml = toml::to_string(&config).unwrap();
        let deserialized: EconomicConfig = toml::from_str(&toml).unwrap();
        assert_eq!(deserialized.reserve.min_ratio, 1.0);
        assert_eq!(deserialized.reserve.target_ratio, 1.2);
        assert_eq!(deserialized.reserve.audit_interval_hours, 24);
        assert_eq!(deserialized.reserve.emergency_threshold, 0.95);
    }

    #[test]
    fn test_economic_invariants() {
        // Test that economic invariants are properly set
        let config = EconomicConfig::default();

        // 1 TokenTorqIngot = 3,600 JouleTorqOre units
        assert_eq!(config.joule_per_ingot, 3600);

        // 1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units
        assert_eq!(config.ingots_per_certificate, 1000);

        // Verify derived calculations
        let joule_per_certificate = config.joule_per_ingot as u64 * config.ingots_per_certificate as u64;
        assert_eq!(joule_per_certificate, 3_600_000);
    }
}