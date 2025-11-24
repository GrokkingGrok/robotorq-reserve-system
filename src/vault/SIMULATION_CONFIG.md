# Simulation Configuration Examples

## Production Mode (Default)
```bash
# Standard production deployment
VAULT_APPROVAL_ENABLED=false
SIMULATION_MODE=false
SIMULATION_TIME_COMPRESSION=1.0
VAULT_DEMURRAGE_RATE_DAILY=0.000137  # 5% annual
```

## Fast Simulation Mode (1000x Speed)
```bash
# For economic modeling - 10 years in ~3.65 days
VAULT_APPROVAL_ENABLED=true
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=1000.0
SIMULATION_ECONOMIC_VARIANCE=0.15
SIMULATION_SCENARIO_ID="monte-carlo-001"
VAULT_DEMURRAGE_RATE_DAILY=0.000137
```

## Stress Test Mode (High Variance)
```bash
# Test system resilience
VAULT_APPROVAL_ENABLED=true
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=100.0
SIMULATION_ECONOMIC_VARIANCE=0.50
VAULT_MAX_CONCURRENT_CONTRACTS=1000
VAULT_MIN_STAKE_RATIO=0.01  # More aggressive approval
```

## Alternative Economics Mode
```bash
# Test different demurrage rates
VAULT_APPROVAL_ENABLED=true
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=500.0
VAULT_DEMURRAGE_RATE_DAILY=0.000685  # 25% annual (aggressive)
SIMULATION_ECONOMIC_VARIANCE=0.05
SIMULATION_SCENARIO_ID="high-demurrage-test"
```

## Import as Package Structure

Your simulator repo could import this vault as:

```rust
// In your simulator/Cargo.toml
[dependencies]
robotorq-vault = { path = "../robotorq-reserve-system/src/vault" }

// Or from git
robotorq-vault = { git = "https://github.com/GrokkingGrok/robotorq-reserve-system", path = "src/vault" }
```

Then configure programmatically:

```rust
use robotorq_vault::{VaultConfig, ShadowDistoVault};

let sim_config = VaultConfig {
    simulation_mode: true,
    time_compression_factor: 1000.0,
    demurrage_rate_daily: 0.000137,
    economic_variance_factor: 0.15,
    simulation_scenario_id: Some("scenario-001".to_string()),
    // ... other fields
};

let vault = ShadowDistoVault::new(nats_client, 1000)
    .with_time_compression(sim_config.time_compression_factor);
```</content>
<parameter name="filePath">c:\Users\Jon\Documents\Project-Asimov\robotorq-reserve-system\src\vault\SIMULATION_CONFIG.md