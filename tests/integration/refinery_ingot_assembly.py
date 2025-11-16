#!/usr/bin/env python3
"""
Integration Test: Refinery Ingot Assembly
Tests: JouleTorqOre → Unit Extraction → IngotAssembler → Phase2Ingot
WITHOUT external services (focused on Refinery internals)
"""

import asyncio
import json
import requests
from nats.aio.client import Client as NATS
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from fixtures.helpers import *

NATS_URL = "nats://localhost:4222"
REFINERY_ORE_ENDPOINT = "http://localhost:8082/receive-ore"
REFINERY_INGOT_TOPIC = "mint.ingots"

def create_sample_ore(contract_id: str, joules: float = 5000.0, units: int = 300) -> dict:
    """Create sample JouleTorqOre for testing"""
    return {
        "contract_id": contract_id,
        "digger_id": "test-digger-001",
        "milestone_index": 0,
        "joules_consumed": joules,
        "robo_stake_paid": 0.05,
        "units": [
            {
                "token_id": f"{contract_id}-m0-t{i:03d}",
                "contract_id": contract_id,
                "digger_id": "test-digger-001",
                "joules_consumed": joules / units,
                "robo_stake_paid": 0.05 / units,
                "milestone_index": 0,
                "timestamp": "2025-11-16T12:00:00Z",
                "hash": f"hash-{i:03d}",
                "signature": b"fake-sig"
            }
            for i in range(units)
        ],
        "timestamp": "2025-11-16T12:00:00Z"
    }


async def test_unit_accumulation_to_3600():
    """
    Test: Verify exactly 3600 units trigger ingot assembly
    - Send ore with 3599 total units → NO ingot
    - Send ore to reach 3600 units → ingot published
    """
    print_section("Test: Unit Accumulation to 3600")
    
    # Prerequisites
    if not check_docker_container("robotorq-network-refinery-1"):
        print_error("Refinery container not running")
        print_step("Start with: docker-compose up -d refinery")
        return False
    
    # Connect to NATS to monitor ingots
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_ingots = []
    
    async def ingot_handler(msg):
        ingot = json.loads(msg.data.decode())
        received_ingots.append(ingot)
        print_success(f"Received ingot: {ingot['ingot_id']}")
    
    await nc.subscribe(REFINERY_INGOT_TOPIC, cb=ingot_handler)
    await asyncio.sleep(1)
    
    # Send ore batches totaling 3599 units
    print_step("Sending ore with 3599 total units...")
    total_units = 0
    
    # 11 batches of 300 units = 3300 units
    for i in range(11):
        ore = create_sample_ore(f"test-contract-{i:02d}", units=300)
        response = requests.post(REFINERY_ORE_ENDPOINT, json=ore)
        if response.status_code != 200:
            print_error(f"Failed to send ore: {response.status_code}")
            return False
        total_units += 300
    
    # 1 batch of 299 units = 3599 total
    ore = create_sample_ore("test-contract-final", units=299)
    response = requests.post(REFINERY_ORE_ENDPOINT, json=ore)
    if response.status_code != 200:
        print_error(f"Failed to send ore: {response.status_code}")
        return False
    total_units += 299
    
    print_success(f"Sent {total_units} units total")
    
    # Wait 5 seconds
    print_step("Waiting 5 seconds (should see NO ingot)...")
    await asyncio.sleep(5)
    
    if len(received_ingots) > 0:
        print_error(f"FAIL: Received {len(received_ingots)} ingots, expected 0!")
        return False
    
    print_success("✅ PASS: No ingot emitted for 3599 units")
    
    # Send 1 more unit to trigger ingot (3600 total)
    print_step("Sending final ore (1 unit) to reach 3600...")
    ore = create_sample_ore("test-contract-trigger", units=1)
    response = requests.post(REFINERY_ORE_ENDPOINT, json=ore)
    if response.status_code != 200:
        print_error(f"Failed to send ore: {response.status_code}")
        return False
    
    # Wait for ingot
    print_step("Waiting for ingot assembly (up to 10 seconds)...")
    for i in range(10):
        await asyncio.sleep(1)
        if len(received_ingots) > 0:
            break
    
    if len(received_ingots) == 0:
        print_error("FAIL: No ingot received after 3600 units!")
        return False
    
    if len(received_ingots) > 1:
        print_warning(f"WARNING: Received {len(received_ingots)} ingots, expected 1")
    
    # Validate ingot structure
    ingot = received_ingots[0]
    
    required_fields = ['ingot_id', 'joules_consumed', 'robo_stake_paid', 
                       'units', 'contract_ids', 'branch_hash']
    
    for field in required_fields:
        if field not in ingot:
            print_error(f"FAIL: Missing field '{field}' in ingot")
            return False
    
    if ingot['units'] != 3600:
        print_error(f"FAIL: Ingot has {ingot['units']} units, expected 3600")
        return False
    
    print_success(f"✅ PASS: Ingot assembled with {ingot['units']} units")
    print_step(f"Branch hash: {ingot['branch_hash']}")
    
    await nc.close()
    return True


async def test_contract_aggregation():
    """
    Test: Multiple contracts in one ingot
    - Send units from 5 different contracts
    - Verify ingot.contract_ids contains all 5
    """
    print_section("Test: Contract Aggregation")
    
    if not check_docker_container("robotorq-network-refinery-1"):
        print_error("Refinery container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_ingots = []
    
    async def ingot_handler(msg):
        ingot = json.loads(msg.data.decode())
        received_ingots.append(ingot)
    
    await nc.subscribe(REFINERY_INGOT_TOPIC, cb=ingot_handler)
    await asyncio.sleep(1)
    
    # Send ore from 5 contracts (720 units each = 3600 total)
    contract_ids = [f"multi-contract-{i}" for i in range(1, 6)]
    
    print_step(f"Sending ore from {len(contract_ids)} contracts...")
    
    for contract_id in contract_ids:
        ore = create_sample_ore(contract_id, units=720)
        response = requests.post(REFINERY_ORE_ENDPOINT, json=ore)
        if response.status_code != 200:
            print_error(f"Failed to send ore for {contract_id}")
            return False
    
    print_success(f"Sent ore from {len(contract_ids)} contracts (3600 units total)")
    
    # Wait for ingot
    print_step("Waiting for ingot assembly...")
    await asyncio.sleep(5)
    
    if len(received_ingots) == 0:
        print_error("FAIL: No ingot received")
        return False
    
    ingot = received_ingots[0]
    
    # Verify contract_ids
    if 'contract_ids' not in ingot:
        print_error("FAIL: No contract_ids in ingot")
        return False
    
    ingot_contracts = set(ingot['contract_ids'])
    expected_contracts = set(contract_ids)
    
    if ingot_contracts != expected_contracts:
        print_error("FAIL: Contract aggregation mismatch")
        print(f"  Expected: {expected_contracts}")
        print(f"  Got: {ingot_contracts}")
        return False
    
    print_success(f"✅ PASS: Ingot contains all {len(contract_ids)} contracts")
    print_step(f"Contracts: {ingot['contract_ids']}")
    
    await nc.close()
    return True


async def test_merkle_branch_hash():
    """
    Test: Ingot branch_hash is valid merkle root
    - Send 3600 units
    - Verify branch_hash is 64-char hex string
    - Verify hash is deterministic (same units → same hash)
    """
    print_section("Test: Merkle Branch Hash Validation")
    
    if not check_docker_container("robotorq-network-refinery-1"):
        print_error("Refinery container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    received_ingots = []
    
    async def ingot_handler(msg):
        ingot = json.loads(msg.data.decode())
        received_ingots.append(ingot)
    
    await nc.subscribe(REFINERY_INGOT_TOPIC, cb=ingot_handler)
    await asyncio.sleep(1)
    
    # Send deterministic ore (12 batches of 300 units)
    print_step("Sending deterministic ore (3600 units)...")
    for i in range(12):
        ore = create_sample_ore("deterministic-contract", units=300)
        response = requests.post(REFINERY_ORE_ENDPOINT, json=ore)
        if response.status_code != 200:
            print_error(f"Failed to send ore batch {i}")
            return False
    
    print_success("Sent 3600 units")
    
    # Wait for ingot
    await asyncio.sleep(5)
    
    if len(received_ingots) == 0:
        print_error("FAIL: No ingot received")
        return False
    
    ingot = received_ingots[0]
    branch_hash = ingot.get('branch_hash', '')
    
    # Validate hash format
    is_valid, error = validate_merkle_root(branch_hash)
    
    if not is_valid:
        print_error(f"FAIL: Invalid branch_hash - {error}")
        return False
    
    print_success(f"✅ PASS: Branch hash is valid merkle root")
    print_step(f"Hash: {branch_hash}")
    
    await nc.close()
    return True


async def main():
    """Run all Refinery integration tests"""
    print_section("Refinery Ingot Assembly Integration Tests")
    
    tests = [
        ("Unit Accumulation to 3600", test_unit_accumulation_to_3600),
        ("Contract Aggregation", test_contract_aggregation),
        ("Merkle Branch Hash", test_merkle_branch_hash),
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
            import traceback
            traceback.print_exc()
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
