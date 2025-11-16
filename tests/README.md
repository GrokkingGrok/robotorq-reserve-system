# RoboTorq Test Suite

Comprehensive testing structure for the RoboTorq network.

## 📁 Directory Structure

```
tests/
├── README.md                    # This file
├── e2e/                         # End-to-end pipeline tests
│   ├── phase2_complete.py       # Phase 2: Digger → Refinery → Mint
│   ├── phase3_complete.py       # Phase 3: Phase2Ingot → Merkle → RT Unit
│   └── full_pipeline.py         # Full: Digger → DistoDam (future)
├── integration/                 # Cross-service integration tests
│   ├── refinery_mint.py         # Refinery → Mint integration
│   ├── mint_distodam.py         # Mint → DistoDam integration
│   └── digger_refinery.py       # Digger → Refinery integration
└── fixtures/                    # Shared test data and helpers
    ├── __init__.py
    ├── helpers.py               # Common test utilities
    └── sample_data.py           # Sample contracts, ores, ingots
```

## 🧪 Test Categories

### 1. Unit Tests (Go)
**Location**: `src/{service}/internal/{component}/*_test.go`  
**Run**: `go test ./... -v`  
**Coverage**: 95%+ target

**Example**:
```bash
cd src/mint
go test ./internal/mint -v -cover
```

### 2. Integration Tests (Python)
**Location**: `tests/integration/`  
**Purpose**: Test interaction between 2-3 services  
**Run**: `python tests/integration/{test_name}.py`

**Example**:
```bash
# Test Refinery → Mint data flow
python tests/integration/refinery_mint.py
```

### 3. End-to-End Tests (Python)
**Location**: `tests/e2e/`  
**Purpose**: Test complete pipelines across all services  
**Run**: `python tests/e2e/{test_name}.py`

**Example**:
```bash
# Test complete Phase 3 pipeline
python tests/e2e/phase3_complete.py
```

## 🚀 Running Tests

### Prerequisites
```bash
# Install Python dependencies
pip install nats-py asyncio

# Start all services
docker-compose up -d

# Verify services are healthy
docker ps
```

### Run All E2E Tests
```bash
# Run all E2E tests sequentially
python -m pytest tests/e2e/ -v

# Or run individually
python tests/e2e/phase2_complete.py
python tests/e2e/phase3_complete.py
```

### Run Integration Tests
```bash
# Run all integration tests
python -m pytest tests/integration/ -v

# Or run individually
python tests/integration/refinery_mint.py
```

### Run Unit Tests (Go)
```bash
# All services
go test ./... -v -cover

# Specific service
cd src/mint
go test ./... -v -cover -race
```

## 📊 Test Output

All tests use standardized colored output:

- ✅ **Green**: Success/passed checks
- ⚠️ **Yellow**: Warnings/non-critical issues
- ❌ **Red**: Failures/errors
- ▶ **Blue**: Info/progress steps

**Example Output**:
```
================================================================================
Phase 3 E2E Test: Complete Proof Chain
================================================================================

▶ Checking prerequisites...
✅ NATS is running
✅ Mint is running

▶ Publishing 1000 Phase2Ingots...
✅ Published 1000 Phase2Ingots

▶ Waiting for Phase3RoboTorqUnit...
✅ Phase3RoboTorqUnit received!
   Unit ID: RT-20251116-191546.769151
   Merkle Root: b68edc2ff2403ede...2457f4063358efb7

✅ ✨ PHASE 3 E2E TEST PASSED! ✨
```

## 🔍 Debugging Failed Tests

### Check Service Logs
```bash
# Mint logs
docker logs robotorq-network-mint-1 --tail 100

# Refinery logs
docker logs robotorq-network-refinery-1 --tail 100

# NATS logs
docker logs robotorq-network-nats-1 --tail 100
```

### Check NATS Subscriptions
```bash
# Install NATS CLI
nats sub ">" --server=localhost:4222

# Monitor specific topic
nats sub "mint.ingots" --server=localhost:4222
nats sub "distodam.units" --server=localhost:4222
```

### Restart Services
```bash
# Restart specific service
docker-compose restart mint

# Rebuild and restart
docker-compose build mint
docker-compose up -d mint

# Full reset
docker-compose down
docker-compose up -d
```

## 📝 Writing New Tests

### E2E Test Template
```python
#!/usr/bin/env python3
"""
E2E Test: {Pipeline Name}
Tests: {Component A} → {Component B} → {Component C}
"""

import asyncio
from nats.aio.client import Client as NATS

# Test configuration
NATS_URL = "nats://localhost:4222"
TIMEOUT = 30  # seconds

class Colors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'

def print_success(msg):
    print(f"{Colors.OKGREEN}✅ {msg}{Colors.ENDC}")

def print_error(msg):
    print(f"{Colors.FAIL}❌ {msg}{Colors.ENDC}")

def print_step(msg):
    print(f"{Colors.OKBLUE}▶ {msg}{Colors.ENDC}")

async def main():
    print_step("Starting E2E test...")
    
    # 1. Setup
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # 2. Test logic
    # ...
    
    # 3. Cleanup
    await nc.close()
    print_success("Test passed!")

if __name__ == "__main__":
    asyncio.run(main())
```

## 🎯 Test Coverage Goals

- **Unit Tests (Go)**: 95%+ coverage
- **Integration Tests**: All critical service pairs
- **E2E Tests**: All major pipelines (Phase 2, Phase 3, full)

## 📚 Related Documentation

- **Architecture**: `src/{service}/{SERVICE}_ARCHITECTURE.md`
- **Development Workflow**: `.github/copilot-instructions.md`
- **Proof Chain**: `PROOF_CHAIN_ARCHITECTURE.md`

---

*"Test early, test often, test everything."* 🧪
