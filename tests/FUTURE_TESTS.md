# Future Test Expansions

**Status**: Documented for Phase 5+ implementation  
**Created**: November 16, 2025  
**Current Phase**: Phase 4 (Cryptography)

---

## Overview

This document captures planned test infrastructure expansions that are **NOT** blocking for Phase 4 (crypto implementation). These should be implemented after Phase 4 is complete and before production deployment.

---

## Test Tier Priorities

### ✅ Completed (Phase 3)
- **Unit Tests (Go)**: 70% - Service-level tests in `*_test.go`
- **Integration Tests (Python)**: 20% - Service-to-service via NATS/HTTP
- **E2E Tests (Python)**: 10% - Full pipeline validation

### 🔄 Phase 5: Chaos & Stress Testing

#### 1. Chaos Tests (`tests/chaos/`)

**Purpose**: Validate service resilience to failures

##### `tests/chaos/nats_disconnect.py`
```python
"""
Test: Services recover from NATS failures
"""

async def test_mint_reconnects_to_nats():
    # Start Mint
    # Kill NATS container
    # Wait 10s
    # Restart NATS
    # Verify: Mint reconnects and processes backlog
    # Ensure: No data loss
```

**Expected Behavior**:
- Services should detect NATS disconnect within 5s
- Automatic reconnection with exponential backoff
- Queue buffering during disconnection (up to 10,000 messages)
- Resume processing after reconnection with no message loss

##### `tests/chaos/service_crash.py`
```python
"""
Test: Pipeline recovers from service crashes
"""

async def test_mint_crash_recovery():
    # Publish 500 Phase2Ingots
    # Kill Mint mid-processing
    # Restart Mint
    # Verify: Mint resumes from checkpoint
    # Verify: No duplicate Phase3Units
```

**Expected Behavior**:
- IngotHashQueue persists state to disk every 100 ingots
- On restart, load checkpoint and resume
- Idempotent processing (same ingot ID → same result)

##### `tests/chaos/network_partition.py`
```python
"""
Test: Services handle network partitions (split-brain)
"""

async def test_refinery_network_partition():
    # Start 2 Refineries
    # Partition network (simulate latency)
    # Send ore to both refineries
    # Heal partition
    # Verify: No duplicate ingots, consistent state
```

**Expected Behavior**:
- Refineries use distributed locks (Redis) for ingot assembly
- Mint deduplicates ingots by ID
- Eventual consistency after partition heals

---

#### 2. Stress Tests (`tests/stress/`)

**Purpose**: Validate performance under load

##### `tests/stress/high_throughput.py`
```python
"""
Test: Mint handles burst traffic
"""

async def test_mint_handles_10k_ingots_per_second():
    # Publish 10,000 Phase2Ingots in 1 second
    # Monitor: Queue depth, memory, CPU
    # Verify: All ingots processed within 60s
    # Verify: No dropped messages
    # Assert: p99 latency < 100ms
```

**Performance Targets**:
- Throughput: 10,000 ingots/sec sustained
- Queue depth: Never exceeds buffer capacity (100,000)
- Memory: <500MB per service
- Latency: p99 < 100ms, p999 < 500ms

##### `tests/stress/concurrent_diggers.py`
```python
"""
Test: Refinery handles 1000 concurrent diggers
"""

async def test_refinery_1000_concurrent_diggers():
    # Spawn 1000 async tasks (simulated diggers)
    # Each sends ore every 1 second
    # Run for 5 minutes
    # Verify: No ingot corruption, all ore processed
    # Monitor: CPU, memory, NATS message rate
```

**Scalability Targets**:
- 1,000 diggers @ 1 ore/sec = 1,000 ore/sec
- 300 units/ore = 300,000 units/sec
- 3600 units/ingot = 83 ingots/sec
- Should handle without performance degradation

##### `tests/stress/long_running.py`
```python
"""
Test: Services run for 24 hours without failure
"""

async def test_24_hour_continuous_operation():
    # Start all services
    # Publish steady stream of data (100 ingots/min)
    # Run for 24 hours
    # Monitor: Memory leaks, file descriptor leaks
    # Verify: No crashes, no performance degradation
```

**Reliability Targets**:
- No memory growth >10% over 24 hours
- No file descriptor leaks
- Graceful log rotation
- Prometheus metrics remain accurate

---

### 🔄 Phase 5: Regression Testing

#### 3. Regression Tests (`tests/regression/`)

**Purpose**: Prevent fixed bugs from reappearing

##### `tests/regression/phase3_deadlock_fix.py`
```python
"""
Regression test for IngotHashQueue deadlock (commit e93b1cd)
Ensures the bug never returns
"""

async def test_ingot_hash_queue_no_deadlock():
    """
    Original bug: GetIngotHashes() blocked forever in select/default
    Fix: Check ctx.Done() before Wait()
    This test verifies context cancellation works correctly
    """
    # Start Mint
    # Publish 500 ingots (not 1000, so queue doesn't flush)
    # Wait 5s
    # Gracefully shutdown Mint (context cancel)
    # Verify: Shutdown completes in <10s (no deadlock)
    
    # If deadlock returns, this test will timeout
```

**How to Add Regression Tests**:
1. When fixing a bug, create `tests/regression/issue_{number}.py`
2. Document original bug behavior
3. Test that fix prevents bug
4. Reference commit SHA and GitHub issue

##### `tests/regression/unit_cross_product_bug.py` (Future)
```python
"""
Regression test for unit calculation bug
Ensures JTU = tokens × joules (not just tokens)
"""

async def test_jtu_count_cross_product():
    # Robot A: 10 tok/s @ 2kW = 20,000 JTU/sec
    # Robot B: 10 tok/s @ 100W = 1,000 JTU/sec
    # Verify: Different JTU counts despite same token count
```

---

### 🔄 Phase 6: Golden Path Tests

#### 4. Golden Path Tests (`tests/golden/`)

**Purpose**: Detect breaking changes in data processing

##### `tests/golden/full_pipeline_snapshot.py`
```python
"""
Golden path: Known input → known output
Records baseline for regression detection
"""

GOLDEN_INPUT = {
    "ore": {
        "contract_id": "golden-contract-001",
        "joules_consumed": 5000.0,
        "units": [
            # Exact 300 units with deterministic data
        ]
    },
    "expected_ingot_hash": "0521434e13fa9bddc71d777d689635f73b1d91df...",
    "expected_unit_id": "RT-20251115-210012.547794",
    "expected_merkle_root": "b68edc2ff2403ede2457f4063358efb7..."
}

async def test_golden_path_unchanged():
    """
    Verify pipeline produces identical output for fixed input
    Any change to hash algorithms or data structures will fail this test
    """
    # Publish GOLDEN_INPUT ore
    # Collect Phase2Ingot
    # Collect Phase3Unit
    # Assert: Hashes match expected values EXACTLY
    
    # If hashes differ:
    # 1. Investigate why (intended change or bug?)
    # 2. Update golden data if change is intentional
    # 3. Document breaking change in changelog
```

**When Golden Tests Fail**:
- ❌ **Breaking change** if unintentional → fix the code
- ✅ **Expected change** (e.g., upgraded hash algorithm) → update golden data
- 📝 **Always document** why golden data changed

---

### 🔄 Phase 7: Advanced Integration Tests

#### 5. Additional Integration Tests

##### `tests/integration/nats_message_ordering.py`
```python
"""
Test: NATS preserves message ordering
"""

async def test_ingot_order_preserved():
    # Publish 100 ingots with sequence numbers
    # Subscribe to Phase3Unit topic
    # Verify: Ingots processed in order (FIFO)
```

##### `tests/integration/refinery_batching.py`
```python
"""
Test: Refinery batching behavior
"""

async def test_ingot_assembler_time_based_flush():
    # Send 1000 units (not 3600)
    # Wait 60 seconds (flush interval)
    # Verify: Ingot NOT emitted (only full batches)
    
async def test_ingot_assembler_size_based_flush():
    # Send exactly 3600 units
    # Verify: Ingot emitted within 5 seconds
```

##### `tests/integration/mint_deduplication.py`
```python
"""
Test: Mint deduplicates ingots by ID
"""

async def test_duplicate_ingot_rejection():
    # Publish same ingot twice
    # Verify: Only 1 ingot counted
    # Verify: Warning logged
```

---

## Test Utilities to Build

### Enhanced Helpers (`tests/fixtures/helpers.py`)

```python
# Chaos testing utilities
def kill_docker_container(name: str):
    """Hard kill a container (simulates crash)"""

def restart_docker_container(name: str):
    """Restart container and wait for health"""

def simulate_network_partition(container1: str, container2: str):
    """Use iptables to block traffic between containers"""

def heal_network_partition(container1: str, container2: str):
    """Remove iptables rules"""

# Performance monitoring
async def measure_throughput(topic: str, duration: int) -> float:
    """Measure messages/sec on a NATS topic"""

async def monitor_memory_usage(container: str, duration: int) -> List[int]:
    """Track memory usage over time"""

async def check_for_memory_leaks(container: str) -> bool:
    """Detect if memory grows unbounded"""

# Load generation
async def generate_load(messages_per_sec: int, duration: int):
    """Generate sustained load for stress testing"""

class LoadGenerator:
    """Advanced load generation with patterns"""
    
    async def burst(self, count: int):
        """Send burst of messages"""
    
    async def steady(self, rate: int, duration: int):
        """Send at constant rate"""
    
    async def ramp_up(self, start_rate: int, end_rate: int, duration: int):
        """Gradually increase load"""
```

---

## Test Execution Strategy

### Phase 5 Testing Workflow

```bash
# 1. Unit tests (always run)
go test ./... -v

# 2. Integration tests (always run)
python tests/integration/mint_phase3_pipeline.py
python tests/integration/refinery_ingot_assembly.py

# 3. E2E tests (always run)
python tests/e2e/phase3_complete.py

# 4. Regression tests (always run)
python tests/regression/phase3_deadlock_fix.py

# 5. Chaos tests (pre-deployment)
python tests/chaos/nats_disconnect.py
python tests/chaos/service_crash.py

# 6. Stress tests (weekly)
python tests/stress/high_throughput.py

# 7. Long-running tests (monthly)
python tests/stress/long_running.py
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/comprehensive-tests.yml
name: Comprehensive Testing

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  unit-tests:
    # Go unit tests (always)
    
  integration-tests:
    # Python integration tests (always)
    
  e2e-tests:
    # Python E2E tests (always)
    
  regression-tests:
    # Regression suite (always)
    
  chaos-tests:
    # Chaos tests (on main branch only)
    if: github.ref == 'refs/heads/main'
    
  stress-tests:
    # Stress tests (scheduled only)
    if: github.event_name == 'schedule'
```

---

## Documentation Requirements

For each test category, document:

1. **Purpose**: What does this test validate?
2. **Expected Behavior**: What should happen?
3. **Failure Scenarios**: What does failure look like?
4. **Recovery Steps**: How to fix when test fails?

---

## Metrics & Observability

### Test Metrics to Track

- **Coverage**: Unit (>95%), Integration (>80%), E2E (100% critical paths)
- **Flakiness**: Tests should pass 99.9% when code is correct
- **Duration**: Integration <10s, E2E <60s, Stress <5min
- **Frequency**: Unit (every commit), E2E (every PR), Stress (weekly)

### Dashboards

Create Grafana dashboard: **"Test Health"**
- Test pass/fail rates over time
- Test duration trends
- Flaky test detection
- Coverage trends

---

## Migration from Manual Testing

### Current State (Phase 3)
- ✅ Unit tests: Automated in CI
- ✅ Integration tests: Manual local execution
- ✅ E2E tests: Manual local execution
- ❌ Chaos tests: Not implemented
- ❌ Stress tests: Not implemented
- ❌ Regression tests: Not implemented

### Target State (Phase 6)
- ✅ All tests automated in CI
- ✅ Chaos tests run on every merge to main
- ✅ Stress tests run weekly
- ✅ Regression suite prevents bug reintroduction
- ✅ Golden path tests catch breaking changes

---

## When to Implement

### Phase 4 (Current - Crypto Implementation)
- ❌ **Do NOT implement** new test categories
- ✅ **Do implement** crypto-specific integration tests:
  - `tests/integration/falcon_signature_validation.py`
  - `tests/integration/sphincs_ledger_signing.py`

### Phase 5 (Post-Crypto)
- ✅ Implement chaos tests (3-5 days)
- ✅ Implement regression tests (2 days)
- ✅ Implement basic stress tests (2-3 days)

### Phase 6 (Pre-Production)
- ✅ Implement golden path tests (2 days)
- ✅ Implement advanced stress tests (1 week)
- ✅ 24-hour stability tests (1 week)

---

## Priority Order

1. **Regression tests** (HIGH) - Prevent backsliding
2. **Chaos tests** (HIGH) - Production readiness
3. **Stress tests** (MEDIUM) - Scalability validation
4. **Golden path tests** (LOW) - Paranoid safety net

---

## Questions to Resolve Before Implementation

1. **Chaos Testing**:
   - Use Chaos Mesh or custom scripts?
   - Test in dedicated environment or CI?

2. **Stress Testing**:
   - What's the target throughput? (1M robots × 20k JTU/sec = ?)
   - Load generation: Real diggers or simulators?

3. **Golden Path**:
   - How often to update golden data?
   - Store in Git or separate artifact storage?

4. **CI Resources**:
   - GitHub Actions sufficient for stress tests?
   - Need dedicated test infrastructure?

---

**Next Step**: Complete Phase 4 (crypto), then revisit this document and implement Phase 5 tests.

*"Test what you fear, fear what you don't test."* 🧪🔥
