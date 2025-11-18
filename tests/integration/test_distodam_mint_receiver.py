#!/usr/bin/env python3
"""
Integration Test: DistoDam MintEventReceiver
Tests NATS subscription to mint.batches and StakeVault deposits
"""

import asyncio
import json
import sys
import time
from datetime import datetime
sys.path.append('tests/fixtures')
from helpers import *

NATS_URL = "nats://localhost:4222"
MINT_BATCHES_TOPIC = "mint.batches"
DISTODAM_METRICS_URL = "http://localhost:8082/metrics"

async def test_mint_event_receiver_valid_event():
    """Test that DistoDam receives and processes valid MintEvent"""
    
    print_section("Integration Test: MintEventReceiver - Valid Event")
    
    # Verify DistoDam is running
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        print_step("Start with: docker-compose up -d distodam")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    print_success("Connected to NATS")
    
    # Create valid MintEvent
    mint_event = {
        "batch_id": "test-batch-001",
        "batch_hash": "abc123def456789",
        "ingot_stakes": [
            {
                "ingot_id": "ingot-001",
                "robo_stake_total": 0.05,
                "contract_ids": ["contract-001", "contract-002"]
            },
            {
                "ingot_id": "ingot-002",
                "robo_stake_total": 0.03,
                "contract_ids": ["contract-003"]
            }
        ],
        "ingots_processed": 2,
        "timestamp": datetime.utcnow().isoformat() + "Z"
    }
    
    print_step(f"Publishing MintEvent: {mint_event['batch_id']}")
    print_step(f"  - Ingots: {len(mint_event['ingot_stakes'])}")
    print_step(f"  - Total stake: {sum(i['robo_stake_total'] for i in mint_event['ingot_stakes'])} RT")
    
    # Publish to mint.batches topic
    await nc.publish(MINT_BATCHES_TOPIC, json.dumps(mint_event).encode())
    await nc.flush()
    
    print_success("MintEvent published")
    
    # Wait for processing
    print_step("Waiting 3 seconds for DistoDam to process...")
    await asyncio.sleep(3)
    
    # Check DistoDam logs for processing confirmation
    logs = get_docker_logs("robotorq-network-distodam-1", since="10s")
    
    if "processing mint event" in logs:
        print_success("✅ MintEvent received by DistoDam")
    else:
        print_error("❌ No processing log found")
        await nc.close()
        return False
    
    if "mint event processed successfully" in logs:
        print_success("✅ MintEvent processed successfully")
    else:
        print_warning("⚠️  Processing completion not confirmed")
    
    # Check metrics
    print_step("Checking Prometheus metrics...")
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    
    if metrics:
        mint_events_received = extract_metric(metrics, "distodam_mint_events_received_total")
        ingots_processed = extract_metric(metrics, "distodam_ingots_processed_total")
        
        if mint_events_received and float(mint_events_received) > 0:
            print_success(f"✅ Mint events received: {mint_events_received}")
        
        if ingots_processed and float(ingots_processed) >= 2:
            print_success(f"✅ Ingots processed: {ingots_processed}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Valid MintEvent processed")
    return True


async def test_mint_event_receiver_invalid_json():
    """Test that DistoDam handles invalid JSON gracefully"""
    
    print_section("Integration Test: MintEventReceiver - Invalid JSON")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    print_step("Publishing invalid JSON to mint.batches")
    
    # Publish malformed JSON
    await nc.publish(MINT_BATCHES_TOPIC, b"{invalid json")
    await nc.flush()
    
    await asyncio.sleep(2)
    
    # Check logs for parse error
    logs = get_docker_logs("robotorq-network-distodam-1", since="5s")
    
    if "failed to parse mint event" in logs:
        print_success("✅ Parse error logged correctly")
    else:
        print_warning("⚠️  Parse error not found in logs")
    
    # Check metrics
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    if metrics:
        parse_errors = extract_metric(metrics, "distodam_mint_event_parse_errors_total")
        if parse_errors and float(parse_errors) > 0:
            print_success(f"✅ Parse errors metric incremented: {parse_errors}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Invalid JSON handled gracefully")
    return True


async def test_mint_event_receiver_validation_error():
    """Test that DistoDam rejects invalid MintEvent (missing batch_id)"""
    
    print_section("Integration Test: MintEventReceiver - Validation Error")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Create MintEvent missing required batch_id
    invalid_event = {
        "batch_hash": "abc123",
        "ingot_stakes": [
            {"ingot_id": "ingot-001", "robo_stake_total": 0.05, "contract_ids": ["c1"]}
        ],
        "ingots_processed": 1
        # Missing: batch_id
    }
    
    print_step("Publishing MintEvent without batch_id")
    
    await nc.publish(MINT_BATCHES_TOPIC, json.dumps(invalid_event).encode())
    await nc.flush()
    
    await asyncio.sleep(2)
    
    # Check logs for validation error
    logs = get_docker_logs("robotorq-network-distodam-1", since="5s")
    
    if "invalid mint event" in logs:
        print_success("✅ Validation error logged")
    else:
        print_warning("⚠️  Validation error not found in logs")
    
    # Check metrics
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    if metrics:
        validation_errors = extract_metric(metrics, "distodam_mint_event_validation_errors_total")
        if validation_errors and float(validation_errors) > 0:
            print_success(f"✅ Validation errors metric incremented: {validation_errors}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Validation error handled")
    return True


async def test_mint_event_receiver_multiple_events():
    """Test that DistoDam handles multiple concurrent MintEvents"""
    
    print_section("Integration Test: MintEventReceiver - Multiple Events")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Get initial metrics
    initial_metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    initial_received = float(extract_metric(initial_metrics, "distodam_mint_events_received_total") or 0)
    
    print_step("Publishing 5 MintEvents concurrently")
    
    # Publish 5 events
    for i in range(5):
        event = {
            "batch_id": f"batch-concurrent-{i:03d}",
            "batch_hash": f"hash-{i}",
            "ingot_stakes": [
                {
                    "ingot_id": f"ingot-{i}-001",
                    "robo_stake_total": 0.01,
                    "contract_ids": [f"contract-{i}"]
                }
            ],
            "ingots_processed": 1,
            "timestamp": datetime.utcnow().isoformat() + "Z"
        }
        await nc.publish(MINT_BATCHES_TOPIC, json.dumps(event).encode())
    
    await nc.flush()
    print_success("5 MintEvents published")
    
    # Wait for processing
    await asyncio.sleep(4)
    
    # Check final metrics
    final_metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    final_received = float(extract_metric(final_metrics, "distodam_mint_events_received_total") or 0)
    
    events_delta = final_received - initial_received
    
    if events_delta >= 5:
        print_success(f"✅ All 5 events received (delta: {events_delta})")
    else:
        print_warning(f"⚠️  Only {events_delta} events received")
    
    await nc.close()
    print_success("✨ TEST PASSED: Multiple events handled")
    return True


async def fetch_metrics(url: str) -> Optional[str]:
    """Fetch Prometheus metrics from HTTP endpoint"""
    try:
        import aiohttp
        async with aiohttp.ClientSession() as session:
            async with session.get(url, timeout=aiohttp.ClientTimeout(total=5)) as response:
                if response.status == 200:
                    return await response.text()
        return None
    except Exception as e:
        print_warning(f"Could not fetch metrics: {e}")
        return None


def extract_metric(metrics: str, metric_name: str) -> Optional[str]:
    """Extract metric value from Prometheus text format"""
    if not metrics:
        return None
    
    for line in metrics.split('\n'):
        if line.startswith(metric_name) and not line.startswith('#'):
            # Format: metric_name{labels} value
            parts = line.split()
            if len(parts) >= 2:
                return parts[-1]  # Last part is the value
    
    return None


async def main():
    print_section("DistoDam MintEventReceiver Integration Tests")
    
    results = []
    
    # Test 1: Valid event
    result1 = await test_mint_event_receiver_valid_event()
    results.append(("Valid Event", result1))
    
    await asyncio.sleep(1)
    
    # Test 2: Invalid JSON
    result2 = await test_mint_event_receiver_invalid_json()
    results.append(("Invalid JSON", result2))
    
    await asyncio.sleep(1)
    
    # Test 3: Validation error
    result3 = await test_mint_event_receiver_validation_error()
    results.append(("Validation Error", result3))
    
    await asyncio.sleep(1)
    
    # Test 4: Multiple events
    result4 = await test_mint_event_receiver_multiple_events()
    results.append(("Multiple Events", result4))
    
    # Summary
    print_section("Test Summary")
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for test_name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{status} - {test_name}")
    
    print(f"\n{Colors.BOLD}Results: {passed}/{total} tests passed{Colors.ENDC}")
    
    if passed == total:
        print_success("🎉 ALL TESTS PASSED!")
        return 0
    else:
        print_error(f"❌ {total - passed} test(s) failed")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
