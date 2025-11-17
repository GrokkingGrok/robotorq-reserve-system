# RoboTorq Helper Scripts

Simple Python scripts to interact with RoboTorq services via HTTP APIs.

## Prerequisites

```bash
pip install requests
```

## Usage

### Digger Operations

```bash
# Create a contract
python scripts/create_contract.py

# Create multiple contracts
python scripts/create_contract.py --count 10

# Stake RoboTorq
python scripts/stake.py --amount 0.05

# Execute a contract
python scripts/execute_contract.py --contract-id my-contract-001

# Full workflow: create + stake + execute
python scripts/run_contract.py --duration 3
```

### Mint Operations

```bash
# Check Mint health
python scripts/check_mint.py

# Get Mint public key
python scripts/get_mint_pubkey.py

# Verify a Phase3 unit signature
python scripts/verify_signature.py --unit-id <unit-id>

# Verify merkle proof
python scripts/verify_merkle.py --proof-type token --token-id <token-id>
```

### Monitoring

```bash
# Check all services health
python scripts/health_check.py

# Monitor ingot count
python scripts/monitor_ingots.py

# Watch for Phase3 units
python scripts/watch_phase3.py
```

## Configuration

Default service URLs (edit scripts to change):
- Digger: http://localhost:9000
- Mint Verification API: http://localhost:8084
- NATS: nats://localhost:4222
