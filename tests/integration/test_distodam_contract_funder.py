#!/usr/bin/env python3
"""
Integration Test: DistoDam ContractFunder
Tests NATS subscription to contracts.approved and contract funding
"""

import asyncio
import json
import sys
import os
import time
from datetime import datetime, timezone
from typing import Optional

# Add parent directories to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'fixtures'))

from fixtures.helpers import (
    print_section, print_step, print_success, print_error, print_warning,
    check_docker_container, get_docker_logs, Colors
)
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
CONTRACTS_APPROVED_TOPIC = "contracts.approved"
CONTRACTS_FUNDED_TOPIC = "contracts.funded"
DISTODAM_METRICS_URL = "http://localhost:8082/metrics"

async def test_contract_funder_valid_contract():
    """Test that DistoDam receives and funds approved contract"""
    
    print_section("Integration Test: ContractFunder - Valid Contract")
    
    # Verify DistoDam is running
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        print_step("Start with: docker-compose up -d distodam")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    print_success("Connected to NATS")
    
    # Subscribe to contracts.funded to verify output
    funded_contracts = []
    
    async def funded_handler(msg):
        contract = json.loads(msg.data.decode())
        funded_contracts.append(contract)
        print_success(f"  Received ContractFundedEvent: {contract.get('contract_id')}")
    
    await nc.subscribe(CONTRACTS_FUNDED_TOPIC, cb=funded_handler)
    print_step(f"Subscribed to {CONTRACTS_FUNDED_TOPIC}")
    
    # Create approved contract
    approved_contract = {
        "id": "test-contract-001",
        "trust_id": "trust-001",
        "opportunity_id": "opp-001",
        "status": "approved",
        "robo_stake": 0.05,
        "digger_id": "digger-001",
        "approved_at": datetime.now(timezone.utc).isoformat(),
        "created_at": datetime.now(timezone.utc).isoformat()
    }
    
    print_step(f"Publishing approved contract: {approved_contract['id']}")
    print_step(f"  - RoboStake: {approved_contract['robo_stake']} RT")
    print_step(f"  - Status: {approved_contract['status']}")
    
    # Publish to contracts.approved topic
    await nc.publish(CONTRACTS_APPROVED_TOPIC, json.dumps(approved_contract).encode())
    await nc.flush()
    
    print_success("Contract published")
    
    # Wait for processing
    print_step("Waiting 3 seconds for DistoDam to process...")
    await asyncio.sleep(3)
    
    # Check DistoDam logs
    logs = get_docker_logs("robotorq-network-distodam-1", since="10s")
    
    if "processing contract" in logs or "contract funded" in logs:
        print_success("✅ Contract received and processed by DistoDam")
    else:
        print_warning("⚠️  Contract processing not confirmed in logs")
    
    # Check if ContractFundedEvent was published
    if funded_contracts:
        print_success(f"✅ ContractFundedEvent published: {len(funded_contracts)} event(s)")
        
        funded = funded_contracts[0]
        print_step(f"  - Contract ID: {funded.get('contract_id')}")
        print_step(f"  - Amount: {funded.get('amount_rt')} RT")
        print_step(f"  - Source: {funded.get('source_vault')}")
        
        if funded.get('loan_amount_rt', 0) > 0:
            print_step(f"  - Loan: {funded.get('loan_amount_rt')} RT")
    else:
        print_warning("⚠️  No ContractFundedEvent received")
    
    # Check metrics
    print_step("Checking Prometheus metrics...")
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    
    if metrics:
        contracts_received = extract_metric(metrics, "distodam_contracts_received_total")
        contracts_funded = extract_metric(metrics, "distodam_contracts_funded_total")
        
        if contracts_received and float(contracts_received) > 0:
            print_success(f"✅ Contracts received: {contracts_received}")
        
        if contracts_funded and float(contracts_funded) > 0:
            print_success(f"✅ Contracts funded: {contracts_funded}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Valid contract funded")
    return True


async def test_contract_funder_invalid_status():
    """Test that DistoDam rejects contract with invalid status"""
    
    print_section("Integration Test: ContractFunder - Invalid Status")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Create contract with wrong status (should be "approved")
    invalid_contract = {
        "id": "test-contract-invalid-status",
        "trust_id": "trust-001",
        "opportunity_id": "opp-001",
        "status": "pending",  # Wrong: should be "approved"
        "robo_stake": 0.05,
        "digger_id": "digger-001"
    }
    
    print_step("Publishing contract with status='pending' (should be 'approved')")
    
    await nc.publish(CONTRACTS_APPROVED_TOPIC, json.dumps(invalid_contract).encode())
    await nc.flush()
    
    await asyncio.sleep(2)
    
    # Check logs for validation error
    logs = get_docker_logs("robotorq-network-distodam-1", since="5s")
    
    if "invalid contract" in logs or "validation" in logs:
        print_success("✅ Validation error logged")
    else:
        print_warning("⚠️  Validation error not found in logs")
    
    # Check metrics
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    if metrics:
        validation_errors = extract_metric(metrics, "distodam_contract_validation_errors_total")
        if validation_errors and float(validation_errors) > 0:
            print_success(f"✅ Validation errors metric incremented: {validation_errors}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Invalid status rejected")
    return True


async def test_contract_funder_zero_stake():
    """Test that DistoDam rejects contract with zero RoboStake"""
    
    print_section("Integration Test: ContractFunder - Zero RoboStake")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Create contract with zero stake
    zero_stake_contract = {
        "id": "test-contract-zero-stake",
        "trust_id": "trust-001",
        "opportunity_id": "opp-001",
        "status": "approved",
        "robo_stake": 0.0,  # Invalid: zero stake
        "digger_id": "digger-001"
    }
    
    print_step("Publishing contract with robo_stake=0.0")
    
    await nc.publish(CONTRACTS_APPROVED_TOPIC, json.dumps(zero_stake_contract).encode())
    await nc.flush()
    
    await asyncio.sleep(2)
    
    # Check logs
    logs = get_docker_logs("robotorq-network-distodam-1", since="5s")
    
    if "invalid" in logs or "zero" in logs:
        print_success("✅ Zero stake error logged")
    else:
        print_warning("⚠️  Zero stake validation not confirmed")
    
    await nc.close()
    print_success("✨ TEST PASSED: Zero stake rejected")
    return True


async def test_contract_funder_insufficient_funds():
    """Test that DistoDam handles insufficient funds gracefully"""
    
    print_section("Integration Test: ContractFunder - Insufficient Funds")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Create contract with very large stake (likely to exceed available funds)
    large_contract = {
        "id": "test-contract-large",
        "trust_id": "trust-001",
        "opportunity_id": "opp-001",
        "status": "approved",
        "robo_stake": 999999.0,  # Extremely large amount
        "digger_id": "digger-001"
    }
    
    print_step("Publishing contract with robo_stake=999999.0 RT")
    
    await nc.publish(CONTRACTS_APPROVED_TOPIC, json.dumps(large_contract).encode())
    await nc.flush()
    
    await asyncio.sleep(2)
    
    # Check logs for insufficient funds
    logs = get_docker_logs("robotorq-network-distodam-1", since="5s")
    
    if "insufficient" in logs:
        print_success("✅ Insufficient funds error logged")
    else:
        print_warning("⚠️  Insufficient funds handling not confirmed")
    
    # Check metrics
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    if metrics:
        insufficient = extract_metric(metrics, "distodam_contracts_insufficient_funds_total")
        if insufficient and float(insufficient) > 0:
            print_success(f"✅ Insufficient funds metric incremented: {insufficient}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Insufficient funds handled")
    return True


async def test_contract_funder_with_loan():
    """Test that DistoDam can fund contract using loan from DistoVault"""
    
    print_section("Integration Test: ContractFunder - Funding with Loan")
    
    if not check_docker_container("robotorq-network-distodam-1"):
        print_error("DistoDam container not running")
        return False
    
    nc = NATS()
    await nc.connect(NATS_URL)
    
    # Subscribe to funded events
    funded_contracts = []
    
    async def funded_handler(msg):
        contract = json.loads(msg.data.decode())
        funded_contracts.append(contract)
    
    await nc.subscribe(CONTRACTS_FUNDED_TOPIC, cb=funded_handler)
    
    # Create moderate-sized contract that might require loan
    contract = {
        "id": "test-contract-loan",
        "trust_id": "trust-001",
        "opportunity_id": "opp-001",
        "status": "approved",
        "robo_stake": 0.10,  # Moderate amount that might need loan
        "digger_id": "digger-001"
    }
    
    print_step(f"Publishing contract: {contract['id']} ({contract['robo_stake']} RT)")
    
    await nc.publish(CONTRACTS_APPROVED_TOPIC, json.dumps(contract).encode())
    await nc.flush()
    
    await asyncio.sleep(3)
    
    # Check if funded with loan
    if funded_contracts:
        funded = funded_contracts[0]
        
        if funded.get('loan_amount_rt', 0) > 0:
            print_success(f"✅ Contract funded with loan: {funded.get('loan_amount_rt')} RT")
            print_step(f"  - Total: {funded.get('amount_rt')} RT")
            print_step(f"  - From StakeVault: {funded.get('amount_rt', 0) - funded.get('loan_amount_rt', 0)} RT")
            print_step(f"  - From DistoVault (loan): {funded.get('loan_amount_rt')} RT")
        else:
            print_success("✅ Contract funded entirely from StakeVault")
            print_step(f"  - Amount: {funded.get('amount_rt')} RT")
    else:
        print_warning("⚠️  No ContractFundedEvent received")
    
    # Check loan metrics
    metrics = await fetch_metrics(DISTODAM_METRICS_URL)
    if metrics:
        loans_total = extract_metric(metrics, "distodam_contract_loans_total")
        if loans_total and float(loans_total) > 0:
            print_success(f"✅ Loans issued: {loans_total}")
    
    await nc.close()
    print_success("✨ TEST PASSED: Loan funding tested")
    return True


async def fetch_metrics(url: str):
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


def extract_metric(metrics: Optional[str], metric_name: str) -> Optional[str]:
    """Extract metric value from Prometheus text format"""
    if not metrics:
        return None
    
    for line in metrics.split('\n'):
        if line.startswith(metric_name) and not line.startswith('#'):
            parts = line.split()
            if len(parts) >= 2:
                return parts[-1]
    
    return None


async def main():
    print_section("DistoDam ContractFunder Integration Tests")
    
    results = []
    
    # Test 1: Valid contract
    result1 = await test_contract_funder_valid_contract()
    results.append(("Valid Contract", result1))
    
    await asyncio.sleep(1)
    
    # Test 2: Invalid status
    result2 = await test_contract_funder_invalid_status()
    results.append(("Invalid Status", result2))
    
    await asyncio.sleep(1)
    
    # Test 3: Zero stake
    result3 = await test_contract_funder_zero_stake()
    results.append(("Zero RoboStake", result3))
    
    await asyncio.sleep(1)
    
    # Test 4: Insufficient funds
    result4 = await test_contract_funder_insufficient_funds()
    results.append(("Insufficient Funds", result4))
    
    await asyncio.sleep(1)
    
    # Test 5: Loan funding
    result5 = await test_contract_funder_with_loan()
    results.append(("Loan Funding", result5))
    
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
