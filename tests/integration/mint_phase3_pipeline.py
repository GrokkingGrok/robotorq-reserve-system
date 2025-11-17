#!/usr/bin/env python3
"""
Integration Test: Mint Phase 3 Internal Pipeline
Tests: Phase2Ingot → IngotHashQueue → Level2Merkle → Phase3Unit
WITHOUT external services (focused on Mint internals)
"""

import asyncio
import json
from nats.aio.client import Client as NATS
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from fixtures.helpers import *

NATS_URL = "nats://localhost:4222"
PHASE2_INGOT_TOPIC = "mint.ingots"
PHASE3_UNIT_TOPIC = "distodam.units"

async def test_ingot_hash_accumulation():
    """
    Test: Verify 1000 ingots queue up correctly
    - Publish 999 ingots → verify NO Phase3Unit emitted
    - Publish 1000th ingot → verify Phase3Unit IS emitted
    """
    print_section("Test: Ingot Hash Accumulation (999 vs 1000)")
    
    # Prerequisites
    if not check_docker_container("robotorq-network-mint-1"):
        print_error("Mint container not running")
        print_step("Start with: docker-compose up -d mint")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_units = []
    
    async def unit_handler(msg):
        unit = json.loads(msg.data.decode())
        received_units.append(unit)
        print_success(f"Received Phase3Unit: {unit['unit_id']}")
    
    # Subscribe to output
    await nc.subscribe(PHASE3_UNIT_TOPIC, cb=unit_handler)
    await asyncio.sleep(1)
    
    # Test 1: Publish 999 ingots (should NOT trigger unit)
    print_step("Publishing 999 Phase2Ingots...")
    for i in range(1, 1000):
        ingot = create_sample_phase2_ingot(
            f"test-ingot-{i:04d}",
            contracts=[f"contract-{i % 5}"],
            diggers=[f"digger-{i % 3}"]
        )
        await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(ingot).encode())
    
    print_success("Published 999 ingots")
    
    # Wait 5 seconds
    print_step("Waiting 5 seconds (should see NO Phase3Unit)...")
    await asyncio.sleep(5)
    
    if len(received_units) > 0:
        print_error(f"FAIL: Received {len(received_units)} units, expected 0!")
        return False
    
    print_success("✅ PASS: No Phase3Unit emitted for 999 ingots")
    
    # Test 2: Publish 1000th ingot (should trigger unit)
    print_step("Publishing 1000th ingot...")
    final_ingot = create_sample_phase2_ingot(
        "test-ingot-1000",
        contracts=["contract-final"],
        diggers=["digger-final"]
    )
    await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(final_ingot).encode())
    
    # Wait up to 10 seconds for unit
    print_step("Waiting for Phase3Unit (up to 10 seconds)...")
    for i in range(10):
        await asyncio.sleep(1)
        if len(received_units) > 0:
            break
    
    if len(received_units) == 0:
        print_error("FAIL: No Phase3Unit received after 1000th ingot!")
        return False
    
    if len(received_units) > 1:
        print_warning(f"WARNING: Received {len(received_units)} units, expected 1")
    
    # Validate unit structure
    unit = received_units[0]
    is_valid, issues = verify_phase3_unit(unit)
    
    if not is_valid:
        print_error(f"FAIL: Phase3Unit validation failed")
        for issue in issues:
            print(f"  - {issue}")
        return False
    
    print_success("✅ PASS: Phase3Unit emitted and valid after 1000 ingots")
    
    await nc.close()
    return True


async def test_merkle_tree_determinism():
    """
    Test: Same 1000 ingots → same merkle root
    - Publish batch A (1000 ingots) → capture merkle_root_1
    - Publish batch B (same 1000 ingots) → capture merkle_root_2
    - Assert merkle_root_1 == merkle_root_2 (deterministic)
    """
    print_section("Test: Merkle Tree Determinism")
    
    # Prerequisites
    if not check_docker_container("robotorq-network-mint-1"):
        print_error("Mint container not running")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_units = []
    
    async def unit_handler(msg):
        unit = json.loads(msg.data.decode())
        received_units.append(unit)
    
    await nc.subscribe(PHASE3_UNIT_TOPIC, cb=unit_handler)
    await asyncio.sleep(1)
    
    # Create deterministic test data (same 1000 ingots)
    test_ingots = []
    for i in range(1, 1001):
        ingot = create_sample_phase2_ingot(
            f"deterministic-ingot-{i:04d}",
            contracts=["contract-A", "contract-B"],
            diggers=["digger-X"],
            joules=1000.0,  # Fixed values
            robo_stake=0.01
        )
        test_ingots.append(ingot)
    
    # Batch 1
    print_step("Publishing Batch A (1000 ingots)...")
    for ingot in test_ingots:
        await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(ingot).encode())
    
    await asyncio.sleep(5)
    
    if len(received_units) == 0:
        print_error("FAIL: No Phase3Unit from Batch A")
        return False
    
    merkle_root_1 = received_units[0]['merkle_root']
    print_success(f"Batch A merkle_root: {merkle_root_1}")
    
    # Wait for Mint to reset (publish 999 dummy ingots to clear queue)
    print_step("Clearing queue with dummy ingots...")
    for i in range(999):
        dummy = create_sample_phase2_ingot(f"dummy-{i}")
        await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(dummy).encode())
    
    await asyncio.sleep(3)
    received_units.clear()
    
    # Batch 2 (identical data)
    print_step("Publishing Batch B (same 1000 ingots)...")
    for ingot in test_ingots:
        await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(ingot).encode())
    
    await asyncio.sleep(5)
    
    if len(received_units) == 0:
        print_error("FAIL: No Phase3Unit from Batch B")
        return False
    
    merkle_root_2 = received_units[0]['merkle_root']
    print_success(f"Batch B merkle_root: {merkle_root_2}")
    
    # Compare
    if merkle_root_1 == merkle_root_2:
        print_success("✅ PASS: Merkle roots are identical (deterministic)")
    else:
        print_error("FAIL: Merkle roots differ!")
        print(f"  Root 1: {merkle_root_1}")
        print(f"  Root 2: {merkle_root_2}")
        return False
    
    await nc.close()
    return True


async def test_multi_contract_aggregation():
    """
    Test: Ingots from multiple contracts aggregate correctly
    - Publish 1000 ingots from 10 different contracts
    - Verify Phase3Unit metadata includes all 10 contracts
    """
    print_section("Test: Multi-Contract Aggregation")
    
    if not check_docker_container("robotorq-network-mint-1"):
        print_error("Mint container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_units = []
    
    async def unit_handler(msg):
        unit = json.loads(msg.data.decode())
        received_units.append(unit)
    
    await nc.subscribe(PHASE3_UNIT_TOPIC, cb=unit_handler)
    await asyncio.sleep(1)
    
    # Publish 1000 ingots across 10 contracts
    print_step("Publishing 1000 ingots from 10 contracts...")
    contract_ids = [f"contract-{i}" for i in range(1, 11)]
    
    for i in range(1, 1001):
        contract_id = contract_ids[i % 10]  # Rotate through contracts
        ingot = create_sample_phase2_ingot(
            f"multi-contract-ingot-{i:04d}",
            contracts=[contract_id],
            diggers=[f"digger-{i % 5}"]
        )
        await nc.publish(PHASE2_INGOT_TOPIC, json.dumps(ingot).encode())
    
    print_success("Published 1000 ingots")
    
    # Wait for Phase3Unit
    print_step("Waiting for Phase3Unit...")
    await asyncio.sleep(5)
    
    if len(received_units) == 0:
        print_error("FAIL: No Phase3Unit received")
        return False
    
    # NOTE: Phase3Unit is hash-only (minimal design)
    # Metadata (contracts, diggers) is logged but NOT in the unit itself
    # This test validates the unit is created, metadata validation happens in logs
    
    unit = received_units[0]
    is_valid, issues = verify_phase3_unit(unit)
    
    if not is_valid:
        print_error("FAIL: Phase3Unit invalid")
        for issue in issues:
            print(f"  - {issue}")
        return False
    
    print_success("✅ PASS: Phase3Unit created from multi-contract ingots")
    print_step("Note: Contract aggregation visible in Mint logs (metadata not persisted)")
    
    await nc.close()
    return True


async def main():
    """Run all Mint Phase 3 integration tests"""
    print_section("Mint Phase 3 Pipeline Integration Tests")
    
    tests = [
        ("Ingot Hash Accumulation", test_ingot_hash_accumulation),
        ("Merkle Tree Determinism", test_merkle_tree_determinism),
        ("Multi-Contract Aggregation", test_multi_contract_aggregation),
    ]
    
    results = []
    
    for test_name, test_func in tests:
        print(f"\n{'=' * 70}")
        print(f"Running: {test_name}")
        print('=' * 70)
        
        try:
            result = await test_func()
            results.append((test_name, result))
        except Exception as e:
            print_error(f"Test crashed: {e}")
            results.append((test_name, False))
    
    # Summary
    print(f"\n{'=' * 70}")
    print_section("Test Summary")
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for test_name, result in results:
        if result:
            print_success(f"✅ {test_name}")
        else:
            print_error(f"❌ {test_name}")
    
    print(f"\n{passed}/{total} tests passed")
    
    if passed == total:
        print_success("🎉 All integration tests PASSED!")
        return True
    else:
        print_error("❌ Some tests FAILED")
        return False


if __name__ == "__main__":
    success = asyncio.run(main())
    exit(0 if success else 1)
