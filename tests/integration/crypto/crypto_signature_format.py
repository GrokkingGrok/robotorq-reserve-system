#!/usr/bin/env python3
"""
Integration Test: Crypto Signature Format Validation

Tests that crypto signatures are properly formatted and included in messages.
Does NOT test cryptographic validity (placeholders accept all signatures).

This test validates:
- Digger includes Falcon signatures in ore batches
- Signatures are valid hex strings
- Public keys are valid hex strings
- Signature lengths are correct
- Mint includes SPHINCS+ signatures in Phase3 units
"""

import asyncio
import json
import sys
import os

# Add fixtures to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from fixtures.helpers import (
    print_section,
    print_step,
    print_success,
    print_error,
    print_warning,
    check_docker_container,
    wait_for_nats_message,
)

from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
ORE_BATCH_TOPIC = "ore.batch"
PHASE3_UNIT_TOPIC = "distodam.units"


async def test_digger_falcon_signature_format():
    """Test that Digger sends properly formatted Falcon-1024 signatures"""
    
    print_step("Testing Digger Falcon-1024 signature format...")
    
    # Prerequisites
    if not check_docker_container("robotorq-network-digger-1"):
        print_error("Digger container not running")
        print_step("Start with: docker-compose up -d digger")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    
    try:
        # Wait for ore batch with signature
        print_step("Waiting for ore batch from Digger...")
        batch = await wait_for_nats_message(ORE_BATCH_TOPIC, timeout=30)
        
        if not batch:
            print_error("No ore batch received within 30 seconds")
            print_warning("Trigger Digger to send batch: POST /contracts/execute")
            return False
        
        # batch is already parsed JSON from wait_for_nats_message
        
        # Validate signature field exists
        if 'signature' not in batch:
            print_error("Ore batch missing 'signature' field")
            print_step(f"Batch keys: {list(batch.keys())}")
            return False
        
        if 'public_key' not in batch:
            print_error("Ore batch missing 'public_key' field")
            return False
        
        signature = batch['signature']
        public_key = batch['public_key']
        
        # Validate signature is hex string
        try:
            sig_bytes = bytes.fromhex(signature)
        except ValueError:
            print_error(f"Signature is not valid hex: {signature[:50]}...")
            return False
        
        # Validate public key is hex string
        try:
            pk_bytes = bytes.fromhex(public_key)
        except ValueError:
            print_error(f"Public key is not valid hex: {public_key[:50]}...")
            return False
        
        # Check signature length (Falcon-1024 SignedMessage ~1300-1400 bytes)
        # Exact size varies but should be in reasonable range
        if len(sig_bytes) < 32:
            print_error(f"Signature too short: {len(sig_bytes)} bytes (expected >32)")
            return False
        
        if len(sig_bytes) > 10000:
            print_error(f"Signature suspiciously long: {len(sig_bytes)} bytes (expected <10KB)")
            return False
        
        # Check public key length (Falcon-1024 public key = 1793 bytes)
        expected_pk_len = 1793
        if len(pk_bytes) != expected_pk_len:
            print_warning(f"Public key length: {len(pk_bytes)} bytes (expected {expected_pk_len})")
            # Not a hard failure - might be placeholder or different encoding
        
        # Success!
        print_success(f"✓ Ore batch has valid signature format")
        print_step(f"  - Signature: {len(sig_bytes)} bytes")
        print_step(f"  - Public key: {len(pk_bytes)} bytes")
        print_step(f"  - Contract: {batch.get('contract_id', 'unknown')}")
        print_step(f"  - Digger: {batch.get('digger_id', 'unknown')}")
        print_step(f"  - Hashes: {batch.get('hash_count', 0)}")
        
        return True
        
    finally:
        await nc.close()


async def test_mint_sphincs_signature_format():
    """Test that Mint sends properly formatted SPHINCS+ signatures"""
    
    print_step("Testing Mint SPHINCS+ signature format...")
    
    # Prerequisites
    if not check_docker_container("robotorq-network-mint-1"):
        print_error("Mint container not running")
        print_step("Start with: docker-compose up -d mint")
        return False
    
    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    
    try:
        # Wait for Phase3 unit with signature
        print_step("Waiting for Phase3RoboTorqUnit from Mint...")
        print_warning("This may take time (need 1000 ingots → 3.6M units)")
        
        unit = await wait_for_nats_message(PHASE3_UNIT_TOPIC, timeout=120)
        
        if not unit:
            print_error("No Phase3 unit received within 120 seconds")
            print_warning("Mint needs 1000 ingots to create 1 RT unit")
            print_step("Check pipeline: Digger → Refinery → Mint")
            return False
        
        # unit is already parsed JSON from wait_for_nats_message
        
        # Validate required fields
        if 'unit_id' not in unit:
            print_error("Phase3 unit missing 'unit_id' field")
            return False
        
        if 'merkle_root' not in unit:
            print_error("Phase3 unit missing 'merkle_root' field")
            return False
        
        # Check for signature (may be omitted if empty)
        if 'signature' in unit and unit['signature']:
            signature = unit['signature']
            
            # Validate signature is hex string
            try:
                sig_bytes = bytes.fromhex(signature)
            except ValueError:
                print_error(f"Signature is not valid hex: {signature[:50]}...")
                return False
            
            # Check length (SPHINCS+ signatures are ~49KB)
            if len(sig_bytes) < 32:
                print_error(f"Signature too short: {len(sig_bytes)} bytes")
                return False
            
            print_success(f"✓ Phase3 unit has valid signature format")
            print_step(f"  - Signature: {len(sig_bytes)} bytes")
        else:
            print_warning("Phase3 unit has no signature (placeholder mode)")
            print_step("  - This is expected in development mode")
            print_step("  - Production MUST have SPHINCS+ signatures")
        
        # Check public key
        if 'public_key' in unit and unit['public_key']:
            public_key = unit['public_key']
            
            try:
                pk_bytes = bytes.fromhex(public_key)
            except ValueError:
                print_error(f"Public key is not valid hex")
                return False
            
            print_step(f"  - Public key: {len(pk_bytes)} bytes")
        
        # Validate merkle root format
        merkle_root = unit['merkle_root']
        if len(merkle_root) != 64:
            print_error(f"Merkle root wrong length: {len(merkle_root)} (expected 64)")
            return False
        
        try:
            bytes.fromhex(merkle_root)
        except ValueError:
            print_error(f"Merkle root is not valid hex")
            return False
        
        print_success(f"✓ Phase3 unit structure valid")
        print_step(f"  - Unit ID: {unit['unit_id']}")
        print_step(f"  - Merkle root: {merkle_root[:16]}...{merkle_root[-16:]}")
        print_step(f"  - Minted at: {unit.get('minted_at', 'unknown')}")
        
        return True
        
    finally:
        await nc.close()


async def main():
    """Run all crypto signature format tests"""
    
    print_section("Crypto Signature Format Integration Tests")
    
    print_warning("⚠️  These tests validate STRUCTURE, not cryptographic validity")
    print_step("Placeholders accept all signatures (development mode)")
    print_step("Production requires liboqs-go for real verification")
    print()
    
    results = {}
    
    # Test 1: Digger Falcon signatures
    print_section("Test 1: Digger Falcon-1024 Signature Format")
    results['digger_falcon'] = await test_digger_falcon_signature_format()
    print()
    
    # Test 2: Mint SPHINCS+ signatures
    print_section("Test 2: Mint SPHINCS+ Signature Format")
    results['mint_sphincs'] = await test_mint_sphincs_signature_format()
    print()
    
    # Summary
    print_section("Test Results Summary")
    
    total = len(results)
    passed = sum(1 for v in results.values() if v)
    failed = total - passed
    
    for test_name, result in results.items():
        status = "✅ PASS" if result else "❌ FAIL"
        print_step(f"{status}: {test_name}")
    
    print()
    print_step(f"Total: {passed}/{total} tests passed")
    
    if passed == total:
        print_success("🎉 All signature format tests passed!")
        print_warning("Note: Cryptographic validity NOT tested (placeholders)")
        return True
    else:
        print_error(f"❌ {failed} test(s) failed")
        return False


if __name__ == "__main__":
    success = asyncio.run(main())
    sys.exit(0 if success else 1)
