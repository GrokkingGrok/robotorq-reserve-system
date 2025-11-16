# Integration Tests

Integration tests validate **service-to-service interactions** without requiring the full end-to-end pipeline.

## Test Organization

```
tests/integration/
├── mint_phase3_pipeline.py      # Mint internal pipeline tests
├── refinery_ingot_assembly.py   # Refinery internal tests
└── README.md                     # This file
```

## Philosophy

**Integration tests sit between Unit tests (Go) and E2E tests (Python)**:

- **Unit tests (Go)**: Test single functions in isolation (`*_test.go`)
- **Integration tests (Python)**: Test service internals via NATS/HTTP
- **E2E tests (Python)**: Test complete multi-service pipelines

## Running Integration Tests

### Prerequisites

1. **Services running**:
   ```bash
   docker-compose up -d nats mint refinery
   ```

2. **Python dependencies**:
   ```bash
   pip install nats-py requests
   ```

### Run Specific Test

```bash
# Mint pipeline tests
python tests/integration/mint_phase3_pipeline.py

# Refinery tests
python tests/integration/refinery_ingot_assembly.py
```

### Run All Integration Tests

```bash
# With pytest
pytest tests/integration/ -v

# Or manually
python tests/integration/mint_phase3_pipeline.py
python tests/integration/refinery_ingot_assembly.py
```

## Test Descriptions

### Mint Phase 3 Pipeline (`mint_phase3_pipeline.py`)

Tests Mint's internal proof chain processing:

1. **Ingot Hash Accumulation**:
   - Publish 999 ingots → NO Phase3Unit
   - Publish 1000th ingot → Phase3Unit emitted
   - Validates 1000-ingot threshold

2. **Merkle Tree Determinism**:
   - Same 1000 ingots → same merkle root
   - Validates hash stability

3. **Multi-Contract Aggregation**:
   - 1000 ingots from 10 contracts
   - Validates metadata aggregation (logged, not persisted)

### Refinery Ingot Assembly (`refinery_ingot_assembly.py`)

Tests Refinery's ore → ingot pipeline:

1. **Unit Accumulation to 3600**:
   - Send 3599 units → NO ingot
   - Send 3600th unit → ingot emitted
   - Validates 3600-unit threshold

2. **Contract Aggregation**:
   - 3600 units from 5 contracts
   - Validates `contract_ids` array in ingot

3. **Merkle Branch Hash**:
   - Validates branch_hash is 64-char hex
   - Confirms merkle root format

## Key Helpers (from `tests/fixtures/helpers.py`)

### NATS Message Verification

```python
# Wait for specific number of messages
msgs = await verify_nats_message_flow("mint.units", 1, timeout=60)

# Wait for single message
unit = await wait_for_nats_message("distodam.units", timeout=30)
```

### Pipeline Latency Measurement

```python
async def trigger():
    # Publish test data
    pass

latency = await measure_pipeline_latency(
    "mint.ingots",
    "distodam.units",
    trigger
)
print(f"Pipeline took {latency:.2f}s")
```

### Merkle Proof Validation

```python
is_valid, issues = verify_merkle_proof_chain(unit, ingot)
if not is_valid:
    for issue in issues:
        print(f"❌ {issue}")
```

## Writing New Integration Tests

### Template

```python
#!/usr/bin/env python3
import asyncio
import sys
sys.path.append('tests/fixtures')
from helpers import *

async def test_your_feature():
    """Test description"""
    print_section("Test: Your Feature")
    
    # 1. Check prerequisites
    if not check_docker_container("robotorq-network-service-1"):
        print_error("Service not running")
        return False
    
    # 2. Subscribe to output topic
    received = []
    async def handler(msg):
        received.append(json.loads(msg.data.decode()))
    
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    await nc.subscribe("output.topic", cb=handler)
    
    # 3. Trigger action (publish, HTTP POST, etc.)
    # ...
    
    # 4. Wait for result
    await asyncio.sleep(5)
    
    # 5. Validate
    if len(received) == 0:
        print_error("No output received")
        return False
    
    print_success("✅ Test passed")
    await nc.close()
    return True

async def main():
    tests = [
        ("Your Feature", test_your_feature),
    ]
    
    results = []
    for test_name, test_func in tests:
        try:
            result = await test_func()
            results.append((test_name, result))
        except Exception as e:
            print_error(f"Crashed: {e}")
            results.append((test_name, False))
    
    # Summary
    passed = sum(1 for _, r in results if r)
    total = len(results)
    print(f"\n{passed}/{total} tests passed")
    return passed == total

if __name__ == "__main__":
    success = asyncio.run(main())
    exit(0 if success else 1)
```

## Best Practices

1. **Test one thing**: Each test function validates one specific behavior
2. **Fast feedback**: Integration tests should run in <10 seconds each
3. **Isolation**: Reset service state between tests (flush queues)
4. **Clear assertions**: Use helpers for validation, print clear error messages
5. **Deterministic data**: Use fixed test data for reproducibility

## Debugging Failed Tests

### Check Service Logs

```bash
# Mint logs
docker logs robotorq-network-mint-1 --tail 100 -f

# Refinery logs
docker logs robotorq-network-refinery-1 --tail 100 -f
```

### Search Logs for Events

```python
from helpers import get_docker_logs, search_docker_logs

logs = get_docker_logs("robotorq-network-mint-1", since="60s")
if "ingot assembled" in logs:
    print_success("Ingot processing confirmed")
```

### Verify NATS Connection

```bash
# Check NATS is running
docker ps | grep nats

# Test NATS connectivity
nats --server nats://localhost:4222 sub ">"
```

## Integration vs E2E

| Aspect | Integration Tests | E2E Tests |
|--------|------------------|-----------|
| **Scope** | Single service | Full pipeline |
| **Speed** | Fast (<10s) | Slower (30-60s) |
| **Dependencies** | Minimal (NATS + 1 service) | All services |
| **Use Case** | Component validation | User workflow |
| **Example** | "Mint queues 1000 ingots" | "Digger → DistoDam" |

**When to use Integration**: Testing internal logic, edge cases, error handling

**When to use E2E**: Validating complete user workflows, deployments

## Continuous Integration

Integration tests run in CI after unit tests:

```yaml
# .github/workflows/ci.yml
- name: Run Integration Tests
  run: |
    docker-compose up -d
    pip install nats-py requests
    pytest tests/integration/ -v
```

---

**Questions?** See `tests/README.md` for complete testing documentation.
