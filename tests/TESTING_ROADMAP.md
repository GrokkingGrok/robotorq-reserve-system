# RoboTorq Testing Roadmap

**Status**: Integration tests complete, additional test types planned for future phases  
**Current Coverage**: Unit (Go) + Integration (Python) + E2E (Python)

---

## ✅ Completed Test Infrastructure

### Unit Tests (Go) - 70% of coverage
- **Location**: `src/{service}/internal/**/*_test.go`
- **Framework**: Go testing + testify/assert
- **Run**: `go test ./... -v -cover`
- **Coverage**: 95%+ for all services

### Integration Tests (Python) - 20% of coverage
- **Location**: `tests/integration/`
- **Framework**: Python asyncio + nats-py + requests
- **Run**: `python tests/integration/{test_name}.py`
- **Coverage**: Mint Phase 3 pipeline, Refinery ingot assembly

**Tests Created**:
- ✅ `mint_phase3_pipeline.py` - Ingot accumulation, merkle determinism, multi-contract
- ✅ `refinery_ingot_assembly.py` - Unit accumulation, contract aggregation, branch hash

### E2E Tests (Python) - 10% of coverage
- **Location**: `tests/e2e/`
- **Framework**: Python asyncio + nats-py + Docker helpers
- **Run**: `python tests/e2e/{test_name}.py`
- **Coverage**: Complete pipeline validation

**Tests Existing**:
- ✅ `phase2_complete.py` - Digger → Refinery → Mint (Phase 2)
- ✅ `phase3_complete.py` - 1000 ingots → Phase3Unit (Phase 3)
- ✅ `phase2_milestone3.py` - Milestone-specific validation
- ✅ `phase3_milestone1.py` - Phase2Ingot receiver

---

## 📋 Planned Test Types (Future Phases)

### 1. Chaos Tests (`tests/chaos/`) - Phase 5+

**Purpose**: Validate system resilience under adverse conditions

**Priority**: MEDIUM  
**Estimated Time**: 4-6 hours  
**Best Time**: After Phase 4 (crypto) is stable

#### Tests to Create

**`nats_disconnect.py`** (2h):
```python
"""
Test service resilience to NATS failures
"""

async def test_mint_reconnects_to_nats():
    """
    1. Start Mint service
    2. Kill NATS container
    3. Wait 10 seconds
    4. Restart NATS container
    5. Verify Mint reconnects automatically
    6. Verify backlog of messages processed
    """
    pass

async def test_refinery_survives_nats_outage():
    """
    1. Send ore to Refinery
    2. Kill NATS mid-processing
    3. Restart NATS
    4. Verify ingot still assembled and sent
    """
    pass
```

**`high_throughput.py`** (3h):
```python
"""
Stress test: High message volume
"""

async def test_mint_handles_burst():
    """
    1. Publish 10,000 Phase2Ingots rapidly (< 5 seconds)
    2. Verify: No dropped messages
    3. Verify: All ingots processed
    4. Check metrics: Queue depth never exceeds buffer size
    5. Measure: Processing latency under load
    """
    pass

async def test_refinery_concurrent_diggers():
    """
    1. Simulate 100 diggers sending ore simultaneously
    2. Verify: All units queued correctly
    3. Verify: No race conditions in queue manager
    4. Verify: Ingots assembled in order
    """
    pass
```

**`service_restart.py`** (1h):
```python
"""
Test graceful shutdown and restart
"""

async def test_mint_graceful_shutdown():
    """
    1. Start processing 500 ingots
    2. Send SIGTERM to Mint (graceful shutdown)
    3. Verify: In-flight work completes
    4. Verify: No data loss
    5. Restart Mint
    6. Verify: Resumes processing
    """
    pass
```

**Value**: Critical for production readiness, catches edge cases unit tests miss

---

### 2. Regression Tests (`tests/regression/`) - Ongoing

**Purpose**: Prevent fixed bugs from reappearing

**Priority**: MEDIUM  
**Estimated Time**: 1-2 hours per bug fix  
**Best Time**: Immediately after fixing critical bugs

#### Tests to Create

**`phase3_deadlock_fix.py`** (1h):
```python
"""
Regression test for IngotHashQueue deadlock (commit e93b1cd)
Ensures the sync.Cond blocking bug never returns
"""

async def test_ingot_hash_queue_no_deadlock():
    """
    Original bug: GetIngotHashes() blocked forever in select/default
    
    Test:
    1. Start Mint service
    2. Publish 500 ingots (not 1000, queue doesn't flush)
    3. Wait 5 seconds
    4. Gracefully shutdown Mint (context cancel)
    5. Verify: Shutdown completes in <10 seconds (no deadlock)
    6. Verify: Logs show context cancellation, not timeout
    """
    pass
```

**`unit_accumulation_edge_cases.py`** (1h):
```python
"""
Regression tests for unit counting bugs
"""

async def test_exactly_3600_units():
    """
    Edge case: Ore with exactly 3600 units should trigger ingot
    Bug: Off-by-one errors in accumulation
    """
    pass

async def test_zero_unit_ore():
    """
    Edge case: Ore with 0 units should be rejected
    Bug: Division by zero, invalid ingots
    """
    pass
```

**Template for Future Regression Tests**:
```python
"""
Regression test for [bug description] (commit [hash])
"""

async def test_[bug_name]_regression():
    """
    Original bug: [What went wrong]
    
    Test:
    1. [Setup that triggers original bug]
    2. [Action that should work now]
    3. Verify: [Expected correct behavior]
    4. Verify: [No error logs]
    """
    pass
```

**Value**: Prevents rework, builds trust in codebase stability

---

### 3. Golden Path Tests (`tests/golden/`) - Phase 6+

**Purpose**: Detect breaking changes in hashing, serialization, or data flow

**Priority**: LOW (nice-to-have)  
**Estimated Time**: 2-3 hours  
**Best Time**: After system is stable (Phase 6+)

#### Tests to Create

**`full_pipeline_snapshot.py`** (2h):
```python
"""
Golden path: Known input → known output
Records baseline for regression detection
"""

GOLDEN_DATA = {
    "ore": {
        "contract_id": "golden-contract-001",
        "digger_id": "golden-digger-001",
        "milestone_index": 0,
        "joules_consumed": 5000.0,
        "robo_stake_paid": 0.05,
        "units": [
            # Exactly 300 units with deterministic data
            # ...
        ],
        "timestamp": "2025-01-01T00:00:00Z"  # Fixed timestamp
    },
    "expected_ingot_hash": "0521434e13fa9bddc71d777d689635f73b1d91df68b74abcbd97af58a051ff10",
    "expected_unit_merkle_root": "b68edc2ff2403ede721abc123456789012345678901234567890123457f4063358efb7",
    "expected_unit_id": "RT-20250101-000000.000000"  # Deterministic from timestamp
}

async def test_golden_path_unchanged():
    """
    Verify pipeline produces IDENTICAL output for fixed input
    
    1. Publish GOLDEN_DATA ore (deterministic)
    2. Collect ingot from Refinery
    3. Assert: ingot.branch_hash == expected_ingot_hash
    4. Collect Phase3Unit from Mint
    5. Assert: unit.merkle_root == expected_unit_merkle_root
    6. Assert: unit.unit_id matches pattern
    
    Breaking changes detected:
    - Hash algorithm changes (SHA256 → SHA512)
    - Serialization changes (JSON encoding, field order)
    - Merkle tree construction changes (different tree structure)
    - Timestamp formatting changes
    """
    pass
```

**`merkle_tree_reproducibility.py`** (1h):
```python
"""
Validate merkle tree construction is deterministic
"""

async def test_same_input_same_merkle_root():
    """
    1. Build merkle tree from 1000 hashes (Set A)
    2. Rebuild merkle tree from same 1000 hashes (Set B)
    3. Assert: root_A == root_B
    4. Test with different orders (should still match)
    5. Test with duplicate hashes (should handle correctly)
    """
    pass
```

**Value**: Paranoid regression detection, useful for cryptographic components

---

### 4. Performance Benchmarks (`tests/performance/`) - Phase 6+

**Purpose**: Track performance metrics over time, catch regressions

**Priority**: LOW  
**Estimated Time**: 3-4 hours  
**Best Time**: After optimization work (Phase 6)

#### Tests to Create

**`throughput_benchmarks.py`** (2h):
```python
"""
Measure messages/sec throughput for each service
"""

async def benchmark_refinery_ore_processing():
    """
    1. Send 10,000 ore messages
    2. Measure: Messages processed per second
    3. Record: p50, p95, p99 latency
    4. Compare against baseline (saved in metrics)
    
    Target: 300 ore/sec sustained
    """
    pass

async def benchmark_mint_ingot_processing():
    """
    Target: 1000 ingots in <5 seconds
    """
    pass
```

**`memory_profiling.py`** (2h):
```python
"""
Memory usage profiling under load
"""

async def test_mint_memory_under_load():
    """
    1. Process 10,000 ingots
    2. Measure: Peak memory usage
    3. Verify: Memory released after processing
    4. Verify: No memory leaks
    
    Target: <500MB per service
    """
    pass
```

**Value**: Prevents performance regressions, guides optimization efforts

---

## 🎯 Testing Priorities for Phase 4 (Crypto)

### Immediate: Crypto-Specific Tests

**Create**: `tests/integration/crypto_signature_validation.py` (3h)

```python
"""
Integration tests for Falcon-1024 + SPHINCS+ signatures
"""

async def test_digger_signs_ore_with_falcon():
    """
    1. Digger generates Falcon-1024 keypair
    2. Digger signs ore hash with private key
    3. Refinery receives ore
    4. Refinery validates signature with public key
    5. Verify: Signature is valid
    """
    pass

async def test_refinery_rejects_invalid_signatures():
    """
    1. Create ore with tampered signature
    2. Send to Refinery
    3. Verify: Ore rejected
    4. Verify: Error logged
    5. Verify: No ingot created
    """
    pass

async def test_mint_signs_unit_with_sphincs():
    """
    1. Mint assembles Phase3Unit
    2. Mint signs merkle_root with SPHINCS+
    3. DistoDam receives unit
    4. DistoDam validates SPHINCS+ signature
    5. Verify: Signature is valid (archival-grade)
    """
    pass

async def test_signature_verification_performance():
    """
    1. Sign 1000 ore hashes with Falcon-1024
    2. Verify all 1000 signatures
    3. Measure: Time per signature verification
    4. Target: <1ms per verification (Falcon advantage)
    """
    pass
```

**Create**: `tests/e2e/crypto_end_to_end.py` (2h)

```python
"""
Full pipeline with cryptographic verification
"""

async def test_signed_ore_to_signed_unit():
    """
    Complete signed proof chain:
    
    1. Digger signs ore with Falcon-1024
    2. Refinery validates signature, assembles ingot
    3. Refinery signs ingot with Falcon-1024
    4. Mint validates ingot signature, builds merkle tree
    5. Mint signs Phase3Unit with SPHINCS+
    6. DistoDam validates SPHINCS+ signature
    
    Verify:
    - All signatures valid end-to-end
    - Tampered data detected at each stage
    - Performance acceptable (<100ms per stage)
    """
    pass
```

---

## 📊 Test Execution Strategy

### Local Development
```bash
# Quick smoke test (unit only)
go test ./... -v

# Integration tests (requires Docker)
docker-compose up -d
python tests/integration/mint_phase3_pipeline.py
python tests/integration/refinery_ingot_assembly.py

# E2E tests (full pipeline)
python tests/e2e/phase3_complete.py
```

### CI/CD Pipeline
```yaml
# .github/workflows/ci.yml
jobs:
  unit-tests:
    - Go unit tests (all services)
    - Rust unit tests (Digger)
  
  integration-tests:
    - Start services (docker-compose)
    - Run integration tests
    - Collect metrics
  
  e2e-tests:
    - Full pipeline test
    - Performance benchmarks
    - Golden path validation
```

### Production Deployment
```bash
# Smoke test in staging
python tests/e2e/phase3_complete.py --env staging

# Canary deployment validation
python tests/integration/mint_phase3_pipeline.py --env canary

# Full regression suite
pytest tests/ -v --env production
```

---

## 📅 Implementation Timeline

| Phase | Test Type | Estimated Time | Priority |
|-------|-----------|----------------|----------|
| **Phase 4 (NOW)** | Crypto integration tests | 5h | 🔥 CRITICAL |
| **Phase 4 (NOW)** | Crypto E2E test | 2h | 🔥 CRITICAL |
| Phase 5 | Chaos tests (NATS, restart) | 4h | 🟡 MEDIUM |
| Phase 5 | Regression tests (ongoing) | 1-2h per bug | 🟡 MEDIUM |
| Phase 6 | Performance benchmarks | 4h | 🟢 LOW |
| Phase 6 | Golden path tests | 3h | 🟢 LOW |

**Total Remaining**: ~20 hours of test development across future phases

---

## 🎓 Testing Best Practices

### DO:
- ✅ Test one thing per test function
- ✅ Use descriptive test names (what + expected outcome)
- ✅ Clean up resources (close connections, stop containers)
- ✅ Use fixtures for reusable test data
- ✅ Assert on specific values, not just "truthy"
- ✅ Add regression tests for every fixed bug

### DON'T:
- ❌ Test multiple unrelated things in one function
- ❌ Use hardcoded sleeps (use polling with timeout)
- ❌ Ignore test failures ("flaky tests")
- ❌ Test implementation details (test behavior)
- ❌ Skip cleanup (leaves system in bad state)

---

## 🔍 Test Coverage Goals

| Layer | Current | Target | Gap |
|-------|---------|--------|-----|
| Unit Tests (Go) | 95% | 95% | ✅ Met |
| Integration Tests | 60% | 80% | +20% (Chaos, Crypto) |
| E2E Tests | 100% | 100% | ✅ Met |
| Regression Tests | 10% | 50% | +40% (Add per bug) |
| Performance Tests | 0% | 20% | +20% (Phase 6) |

**Overall Coverage**: 70% → 85% (target after Phase 6)

---

## 📝 Documentation

- **Test README**: `tests/README.md` - Overview of all test types
- **Integration README**: `tests/integration/README.md` - How to run integration tests
- **Fixtures README**: `tests/fixtures/README.md` - Shared utilities documentation
- **This Roadmap**: `tests/TESTING_ROADMAP.md` - Long-term testing strategy

---

**Next Steps**:
1. ✅ Complete integration test infrastructure (DONE)
2. 🔄 Implement crypto integration tests (Phase 4 - IN PROGRESS)
3. 📅 Add chaos tests after Phase 4 stabilizes
4. 📅 Add performance benchmarks before production

*"Test what matters, automate what repeats, document what breaks."* 🧪🔬
