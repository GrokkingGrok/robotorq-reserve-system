# Mint Simulation Configuration Examples

## Production Mode (Default)
```bash
# Standard production deployment
MINT_INGOTS_PER_CERT=1000
MINT_BATCH_THRESHOLD_COUNT=10
MINT_BATCH_THRESHOLD_SECONDS=300
MINT_PROOF_INTERVAL_COUNT=100
MINT_PROOF_INTERVAL_SECONDS=3600
SIMULATION_MODE=false
SIMULATION_TIME_COMPRESSION=1.0
```

## Fast Simulation Mode (1000x Speed)
```bash
# For testing certificate and proof generation - simulate years of operation
MINT_INGOTS_PER_CERT=1000
MINT_BATCH_THRESHOLD_COUNT=5      # Smaller batches for faster testing
MINT_BATCH_THRESHOLD_SECONDS=60   # 1 minute windows
MINT_PROOF_INTERVAL_COUNT=20      # Smaller proofs for testing
MINT_PROOF_INTERVAL_SECONDS=600   # 10 minutes
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=1000.0
MINT_INGOT_GENERATION_RATE=50.0   # 50 ingots/second
MINT_BATCH_PROCESSING_DELAY_MS=10  # Minimal delays
MINT_PROOF_SIGNING_DELAY_MS=50
SIMULATION_SCENARIO_ID="mint-throughput-test"
```

## Stress Test Mode (High Volume)
```bash
# Test mint capacity under load
MINT_INGOTS_PER_CERT=1000
MINT_BATCH_THRESHOLD_COUNT=50     # Large batches
MINT_BATCH_THRESHOLD_SECONDS=30   # Short windows
MINT_PROOF_INTERVAL_COUNT=500     # Large proofs
MINT_PROOF_INTERVAL_SECONDS=1800  # 30 minutes
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=100.0
MINT_INGOT_GENERATION_RATE=200.0  # High ingot rate
MINT_BATCH_PROCESSING_DELAY_MS=5   # Minimal delays
MINT_PROOF_SIGNING_DELAY_MS=20
SIMULATION_SCENARIO_ID="mint-stress-test"
```

## Merkle Tree Depth Test
```bash
# Test different merkle tree configurations
MINT_INGOTS_PER_CERT=1000
MINT_BATCH_THRESHOLD_COUNT=10
MINT_BATCH_THRESHOLD_SECONDS=300
MINT_PROOF_INTERVAL_COUNT=100
MINT_PROOF_INTERVAL_SECONDS=3600
MINT_MERKLE_TREE_DEPTH=20         # Deeper tree: 2^20 = 1M leaves
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=500.0
MINT_INGOT_GENERATION_RATE=25.0
SIMULATION_SCENARIO_ID="merkle-depth-test"
```

## Import as Package Structure

Your simulator repo could import this mint as:

```rust
// In your simulator/Cargo.toml
[dependencies]
robotorq-mint = { path = "../robotorq-reserve-system/src/mint" }

// Or from git
robotorq-mint = { git = "https://github.com/GrokkingGrok/robotorq-reserve-system", path = "src/mint" }
```

Then configure programmatically:

```rust
use robotorq_mint::{MintConfig, IngotProcessor};

let sim_config = MintConfig {
    simulation_mode: true,
    time_compression_factor: 1000.0,
    ingot_generation_rate: 50.0,
    batch_threshold_count: 5,
    proof_interval_count: 20,
    simulation_scenario_id: Some("scenario-001".to_string()),
    // ... other fields
};

let processor = IngotProcessor::new(sim_config)
    .with_time_compression(sim_config.time_compression_factor);
```