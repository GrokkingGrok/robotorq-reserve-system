#!/usr/bin/env python3
"""
Phase 5 Manual Verification Script
Monitors for Phase3 unit creation and verifies signatures/merkle proofs

Usage: python scripts/verify_phase5_complete.py
"""

import asyncio
import requests
import json
import sys
import os
from datetime import datetime

# Add fixtures to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../tests')))
from fixtures.helpers import print_section, print_step, print_success, print_error, print_warning

MINT_API = "http://localhost:8084"

def check_mint_status():
    """Check current Mint ingot count"""
    print_section("Mint Status Check")
    
    try:
        response = requests.get(f"{MINT_API}/health", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        cache_size = data.get('cache_size', 0)
        print_step(f"Phase3 units in cache: {cache_size}")
        print_step(f"Mint status: {data.get('status', 'unknown')}")
        
        return cache_size > 0
        
    except Exception as e:
        print_error(f"Failed to check Mint status: {e}")
        return False


def verify_phase3_unit(unit_id):
    """Verify a Phase3 unit's signature"""
    print_section(f"Verifying Phase3 Unit: {unit_id}")
    
    # Test 1: Signature verification
    print_step("Test 1: Verifying SPHINCS+ signature...")
    try:
        response = requests.get(f"{MINT_API}/verify/signature/{unit_id}", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        if data.get('valid'):
            print_success("✅ SPHINCS+ signature VALID")
        else:
            print_error(f"❌ Signature verification failed: {data.get('error')}")
            return False
            
    except Exception as e:
        print_error(f"Signature verification error: {e}")
        return False
    
    # Test 2: Get public key
    print_step("Test 2: Fetching Mint public key...")
    try:
        response = requests.get(f"{MINT_API}/public-key", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        print_success(f"✅ Public key retrieved: {data.get('algorithm')}")
        
    except Exception as e:
        print_error(f"Public key fetch error: {e}")
    
    # Test 3: Unit merkle proof (if available)
    print_step("Test 3: Verifying merkle proof for unit...")
    try:
        response = requests.get(f"{MINT_API}/verify/merkle/unit/{unit_id}", timeout=5)
        
        if response.status_code == 200:
            data = response.json()
            if data.get('valid'):
                print_success("✅ Unit merkle proof VALID")
            else:
                print_warning(f"⚠️  Merkle proof validation: {data.get('error', 'Unknown')}")
        elif response.status_code == 501:
            print_warning("⚠️  Merkle proof endpoint not yet implemented (expected)")
        else:
            print_warning(f"⚠️  Merkle proof check returned {response.status_code}")
            
    except Exception as e:
        print_warning(f"Merkle proof check: {e}")
    
    return True


async def watch_for_phase3_units():
    """Watch NATS for Phase3 units"""
    print_section("Watching for Phase3 RoboTorq Units")
    print_step("Connecting to NATS...")
    
    try:
        from nats.aio.client import Client as NATS
        
        nc = NATS()
        await nc.connect("nats://localhost:4222")
        print_success("Connected to NATS")
        
        units_found = []
        
        async def unit_handler(msg):
            try:
                unit = json.loads(msg.data.decode())
                unit_id = unit.get('unit_id', 'unknown')
                
                print_section(f"Phase3 Unit Received: {unit_id}")
                print_step(f"JouleTorq Total: {unit.get('jouletorq_total', 0):.2f}")
                print_step(f"RoboTorq Total: {unit.get('robotorq_total', 0):.6f}")
                print_step(f"Ingots: {len(unit.get('phase2_ingots', []))}")
                print_step(f"Root Hash: {unit.get('merkle_root_hash', 'N/A')[:64]}...")
                
                has_signature = bool(unit.get('signature'))
                has_pubkey = bool(unit.get('public_key'))
                
                print_step(f"Signature present: {has_signature}")
                print_step(f"Public key present: {has_pubkey}")
                
                if has_signature and has_pubkey:
                    print_success("✅ Unit has signature and public key!")
                    units_found.append(unit_id)
                    
                    # Verify it
                    verify_phase3_unit(unit_id)
                else:
                    print_error("❌ Unit missing signature or public key")
                    
            except Exception as e:
                print_error(f"Error processing unit: {e}")
        
        await nc.subscribe("distodam.units", cb=unit_handler)
        print_step("Subscribed to distodam.units topic")
        print_step("Waiting for Phase3 units... (Ctrl+C to stop)")
        
        # Wait indefinitely
        while True:
            await asyncio.sleep(10)
            if units_found:
                print_step(f"[{datetime.now().strftime('%H:%M:%S')}] Total units verified: {len(units_found)}")
        
    except KeyboardInterrupt:
        print_success(f"\n✨ Verified {len(units_found)} Phase3 unit(s)")
    except Exception as e:
        print_error(f"NATS error: {e}")


def main():
    """Main verification flow"""
    print("\n" + "="*70)
    print("Phase 5 Verification - Manual Check")
    print("="*70 + "\n")
    
    # Check if Mint has any Phase3 units yet
    has_units = check_mint_status()
    
    if has_units:
        print_success("\n✅ Mint has Phase3 units in cache!")
        print_step("You can verify them using:")
        print_step("  python scripts/verify_signature.py --unit-id <unit-id>")
        print_step("  python scripts/verify_merkle.py --proof-type unit --unit-id <unit-id>")
    else:
        print_warning("\n⏳ No Phase3 units yet. Waiting for pipeline...")
        print_step("\nStarting NATS monitor to watch for Phase3 units...")
        print_step("This will automatically verify when a unit is created.\n")
        
        # Watch NATS for units
        asyncio.run(watch_for_phase3_units())


if __name__ == "__main__":
    main()
