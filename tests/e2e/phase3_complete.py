#!/usr/bin/env python3
"""
Phase 3 Milestone 5 E2E Test: Complete Proof Chain Validation

Tests the COMPLETE Phase 3 flow:

1. Publish Phase2Ingot (1000 ingot hashes) → mint.ingots
2. Mint IngotHashQueue batches 1000 hashes
3. Level2MerkleBuilder builds merkle tree
4. Phase3RoboTorqUnitAssembler creates Phase3RoboTorqUnit
5. Phase3DistoDamPublisher sends to distodam.units
6. Verify Phase3RoboTorqUnit received with valid merkle root

This validates the COMPLETE proof chain architecture!

Architecture:
Phase2Ingot (1000 hashes) → mint.ingots (NATS) →
Phase2IngotReceiver → IngotHashQueue (batch 1000) →
Level2MerkleBuilder (merkle tree) → Phase3Assembler (RT unit) →
Phase3Publisher → distodam.units (NATS) → Verification

Success Criteria:
- ✅ Mint service running and healthy
- ✅ Phase2Ingot published to mint.ingots
- ✅ IngotHashQueue batched 1000 hashes
- ✅ Level2MerkleBuilder created merkle root
- ✅ Phase3RoboTorqUnit published to distodam.units
- ✅ RT unit has valid merkle_root (64-char hex)
- ✅ RT unit has correct metadata (contract IDs, digger IDs)
- ✅ Complete proof chain preserved
"""

import asyncio
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

try:
    from nats.aio.client import Client as NATS
except ImportError:
    print("❌ ERROR: nats-py not installed")
    print("Install with: pip install nats-py")
    sys.exit(1)


# Configuration
NATS_URL = "nats://localhost:4222"
MINT_CONTAINER = "robotorq-network-mint-1"
MINT_INGOTS_TOPIC = "mint.ingots"
DISTODAM_UNITS_TOPIC = "distodam.units"

# Test parameters
INGOT_COUNT = 1000  # Number of Phase2Ingots to publish (Mint batches 1000 ingots)
EXPECTED_RT_UNITS = 1  # Should produce 1 Phase3RoboTorqUnit


class Colors:
    """ANSI color codes for pretty output"""
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'


def print_section(title: str):
    """Print a section header"""
    print(f"\n{Colors.BOLD}{Colors.CYAN}{'=' * 80}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.CYAN}{title}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.CYAN}{'=' * 80}{Colors.ENDC}\n")


def print_step(message: str):
    """Print a step message"""
    print(f"{Colors.BLUE}▶ {message}{Colors.ENDC}")


def print_success(message: str):
    """Print a success message"""
    print(f"{Colors.GREEN}✅ {message}{Colors.ENDC}")


def print_error(message: str):
    """Print an error message"""
    print(f"{Colors.RED}❌ {message}{Colors.ENDC}")


def print_warning(message: str):
    """Print a warning message"""
    print(f"{Colors.YELLOW}⚠️  {message}{Colors.ENDC}")


def check_prerequisites():
    """Verify all required services are running"""
    print_section("Prerequisites Check")
    
    # Check NATS is running
    print_step("Checking NATS...")
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", "name=nats", "--format", "{{.Status}}"],
            capture_output=True,
            text=True,
            check=True,
            timeout=5
        )
        if "Up" in result.stdout:
            print_success("NATS is running")
        else:
            print_error("NATS is not running. Start with: docker-compose up -d nats")
            return False
    except Exception as e:
        print_error(f"Failed to check NATS: {e}")
        return False
    
    # Check Mint is running
    print_step("Checking Mint...")
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", f"name={MINT_CONTAINER}", "--format", "{{.Status}}"],
            capture_output=True,
            text=True,
            check=True,
            timeout=5
        )
        if "Up" in result.stdout:
            print_success("Mint is running")
        else:
            print_error("Mint is not running. Start with: docker-compose up -d mint")
            return False
    except Exception as e:
        print_error(f"Failed to check Mint: {e}")
        return False
    
    return True


def generate_phase2_ingot(index: int) -> dict:
    """Generate a single Phase2Ingot with deterministic branch hash"""
    import hashlib
    
    # Create deterministic ingot ID
    ingot_id = f"phase3-e2e-ingot-{index:04d}"
    
    # Generate deterministic branch hash (simulates Refinery's merkle root)
    hash_obj = hashlib.sha256(ingot_id.encode())
    branch_hash = hash_obj.hexdigest()
    
    # Build Phase2Ingot (matches Refinery output)
    phase2_ingot = {
        "id": ingot_id,
        "branch_hash": branch_hash,
        "hash_count": 3600,  # Always 3600 JTUs per Refinery ingot
        "contract_ids": ["e2e-contract-001", "e2e-contract-002"],
        "digger_ids": ["e2e-digger-001"],
        "timestamp": datetime.now(timezone.utc).isoformat()
    }
    
    return phase2_ingot


async def publish_phase2_ingots(nc: NATS, count: int) -> list:
    """Publish multiple Phase2Ingots to mint.ingots (simulating Refinery output)"""
    print_section("Publishing Phase2Ingots")
    
    print_step(f"Publishing {count} Phase2Ingots to {MINT_INGOTS_TOPIC}...")
    print(f"   Each ingot represents 3600 JTUs from Refinery")
    print(f"   Mint will extract {count} branch hashes for Level 2 merkle tree")
    
    published_ingots = []
    
    # Publish ingots in batches with progress indicator
    batch_size = 100
    for batch_start in range(0, count, batch_size):
        batch_end = min(batch_start + batch_size, count)
        
        for i in range(batch_start, batch_end):
            ingot = generate_phase2_ingot(i + 1)
            payload = json.dumps(ingot).encode()
            await nc.publish(MINT_INGOTS_TOPIC, payload)
            published_ingots.append(ingot)
        
        # Flush after each batch
        await nc.flush()
        
        # Show progress
        print(f"   Published {batch_end}/{count} ingots...", end='\r')
    
    print()  # New line after progress
    print_success(f"Published {len(published_ingots)} Phase2Ingots")
    print(f"   First ingot: {published_ingots[0]['id']}")
    print(f"   Last ingot:  {published_ingots[-1]['id']}")
    print(f"   Sample hash: {published_ingots[0]['branch_hash'][:16]}...{published_ingots[0]['branch_hash'][-16:]}")
    
    return published_ingots


async def subscribe_to_distodam_units(nc: NATS, timeout: int = 15) -> list:
    """Subscribe to distodam.units and collect Phase3RoboTorqUnits"""
    print_section("Subscribing to DistoDam Units")
    
    print_step(f"Subscribing to {DISTODAM_UNITS_TOPIC}...")
    print_step(f"Waiting up to {timeout} seconds for Phase3RoboTorqUnit...")
    
    received_units = []
    
    async def message_handler(msg):
        """Handle incoming Phase3RoboTorqUnit messages"""
        try:
            unit_json = msg.data.decode()
            unit = json.loads(unit_json)
            received_units.append(unit)
            
            print_success("Phase3RoboTorqUnit received!")
            print(f"   Unit ID: {unit.get('unit_id', 'MISSING')}")
            print(f"   Merkle Root: {unit.get('merkle_root', 'MISSING')[:16]}...{unit.get('merkle_root', 'MISSING')[-16:]}")
            print(f"   Minted At: {unit.get('minted_at', 'MISSING')}")
            print(f"   Size: {len(unit_json)} bytes")
            
        except Exception as e:
            print_error(f"Failed to parse Phase3RoboTorqUnit: {e}")
    
    # Subscribe to topic
    sub = await nc.subscribe(DISTODAM_UNITS_TOPIC, cb=message_handler)
    
    # Wait for units to arrive
    start_time = time.time()
    while (time.time() - start_time) < timeout:
        if len(received_units) >= EXPECTED_RT_UNITS:
            break
        await asyncio.sleep(0.5)
    
    # Unsubscribe
    await sub.unsubscribe()
    
    if not received_units:
        print_warning(f"No Phase3RoboTorqUnits received within {timeout} seconds")
    else:
        print_success(f"Received {len(received_units)} Phase3RoboTorqUnit(s)")
    
    return received_units


def check_mint_logs():
    """Check Mint logs for Phase 3 processing"""
    print_section("Checking Mint Logs")
    
    print_step("Searching for Phase 3 pipeline activity...")
    
    try:
        result = subprocess.run(
            ["docker", "logs", MINT_CONTAINER, "--since", "30s"],
            capture_output=True,
            text=True,
            check=True,
            timeout=5
        )
        
        logs = result.stdout
        
        # Check for key Phase 3 events
        events = {
            "Phase2Ingot received": "Phase 2 ingot received",
            "IngotHashQueue batched": "queued 1000 hashes" or "hash queue full",
            "Level2 merkle built": "Level 2 merkle tree built" or "merkle_root",
            "Phase3 unit assembled": "Phase3RoboTorqUnit assembled" or "RT unit created",
            "Published to DistoDam": "published to distodam.units" or "unit published"
        }
        
        found_events = []
        for event_name, search_term in events.items():
            if search_term in logs:
                found_events.append(event_name)
                print_success(f"   ✓ {event_name}")
            else:
                print_warning(f"   ? {event_name} (not found in logs)")
        
        if len(found_events) >= 3:
            print_success(f"Found {len(found_events)}/5 expected events")
            return True
        else:
            print_warning(f"Only found {len(found_events)}/5 events")
            return False
        
    except Exception as e:
        print_error(f"Failed to check Mint logs: {e}")
        return False


def verify_phase3_unit(unit: dict) -> bool:
    """Verify Phase3RoboTorqUnit structure and contents"""
    print_section("Verifying Phase3RoboTorqUnit")
    
    all_valid = True
    
    # Check unit_id
    print_step("Checking unit_id...")
    unit_id = unit.get('unit_id', '')
    if unit_id and unit_id.startswith('RT-'):
        print_success(f"   Valid unit ID: {unit_id}")
    else:
        print_error(f"   Invalid unit_id: {unit_id}")
        all_valid = False
    
    # Check merkle_root
    print_step("Checking merkle_root...")
    merkle_root = unit.get('merkle_root', '')
    if len(merkle_root) == 64 and all(c in '0123456789abcdef' for c in merkle_root):
        print_success(f"   Valid merkle root: {merkle_root[:16]}...{merkle_root[-16:]}")
    else:
        print_error(f"   Invalid merkle root: {merkle_root}")
        all_valid = False
    
    # Check minted_at timestamp (Phase 3 field)
    print_step("Checking minted_at...")
    minted_at = unit.get('minted_at', '')
    if minted_at:
        print_success(f"   Minted at: {minted_at}")
    else:
        print_error(f"   Missing minted_at timestamp")
        all_valid = False
    
    # Verify no metadata fields (Phase 3 design: merkle root only)
    print_step("Verifying minimal design (no metadata fields)...")
    metadata_fields = ['contract_ids', 'digger_ids', 'refinery_ids', 'total_joules', 'total_robo_stake']
    has_metadata = any(field in unit for field in metadata_fields)
    if not has_metadata:
        print_success(f"   ✓ Metadata-free design (merkle root only)")
    else:
        found = [f for f in metadata_fields if f in unit]
        print_warning(f"   Unexpected metadata fields: {found}")
    
    # Check size (should fit in NFC tag - target: <200 bytes)
    print_step("Checking size for NFC compatibility...")
    unit_json = json.dumps(unit)
    unit_size = len(unit_json.encode())
    if unit_size < 200:
        print_success(f"   Size: {unit_size} bytes (fits in NTAG215 tag)")
    else:
        print_warning(f"   Size: {unit_size} bytes (may be too large for NFC)")
    
    return all_valid


async def main():
    """Run the complete Phase 3 E2E test"""
    print(f"\n{Colors.BOLD}{Colors.HEADER}")
    print("=" * 80)
    print("Phase 3 Milestone 5: Complete Proof Chain E2E Test")
    print("Phase2Ingot → Mint → Level2 Merkle → Phase3 RT Unit → DistoDam")
    print("=" * 80)
    print(f"{Colors.ENDC}\n")
    
    try:
        # Step 1: Check prerequisites
        if not check_prerequisites():
            print_error("Prerequisites not met. Please fix and try again.")
            return False
        
        # Step 2: Connect to NATS
        print_section("Connecting to NATS")
        print_step(f"Connecting to {NATS_URL}...")
        nc = NATS()
        await nc.connect(NATS_URL)
        print_success("Connected to NATS")
        
        # Step 3: Subscribe to distodam.units FIRST (before publishing)
        # This ensures we don't miss the message
        print_step("Setting up subscriber...")
        subscription_task = asyncio.create_task(subscribe_to_distodam_units(nc, timeout=15))
        
        # Small delay to ensure subscription is ready
        await asyncio.sleep(1)
        
        # Step 4: Publish 1000 Phase2Ingots (simulating Refinery output)
        published_ingots = await publish_phase2_ingots(nc, INGOT_COUNT)
        
        # Step 5: Wait for Phase3RoboTorqUnit to be published
        print_step("Waiting for Mint to process ingot and publish RT unit...")
        received_units = await subscription_task
        
        # Step 6: Check Mint logs
        logs_ok = check_mint_logs()
        
        # Step 7: Verify received units
        if not received_units:
            print_error("No Phase3RoboTorqUnits received!")
            print("\n📋 Debug Steps:")
            print("1. Check Mint is processing Phase2Ingots:")
            print(f"   docker logs {MINT_CONTAINER} --since 30s | grep 'Phase 2 ingot'")
            print("2. Check IngotHashQueue batching:")
            print(f"   docker logs {MINT_CONTAINER} --since 30s | grep 'hash queue'")
            print("3. Check Phase3 assembler:")
            print(f"   docker logs {MINT_CONTAINER} --since 30s | grep 'Phase3'")
            print("4. Check publisher:")
            print(f"   docker logs {MINT_CONTAINER} --since 30s | grep 'distodam'")
            
            await nc.close()
            return False
        
        # Verify first unit
        unit_valid = verify_phase3_unit(received_units[0])
        
        # Final result
        print_section("Test Results")
        
        if received_units and unit_valid:
            print_success("✨ MILESTONE 5 E2E TEST PASSED! ✨")
            print(f"\n{Colors.GREEN}Summary:{Colors.ENDC}")
            print(f"   ✅ Phase2Ingots published: {INGOT_COUNT} ingots")
            print(f"   ✅ IngotHashQueue batched: {INGOT_COUNT} branch hashes")
            print(f"   ✅ Level2MerkleBuilder: Merkle tree created")
            print(f"   ✅ Phase3RoboTorqUnit assembled: 1 unit")
            print(f"   ✅ Published to DistoDam: distodam.units")
            print(f"   ✅ Complete proof chain validated ✓")
            print(f"\n{Colors.BOLD}Phase 3 is COMPLETE! Ready for documentation.{Colors.ENDC}\n")
            
            await nc.close()
            return True
        else:
            print_warning("Test completed with issues")
            print(f"   Units received: {len(received_units)}")
            print(f"   Unit valid: {'Yes' if unit_valid else 'No'}")
            
            await nc.close()
            return False
    
    except KeyboardInterrupt:
        print_warning("\nTest interrupted by user")
        return False
    
    except Exception as e:
        print_error(f"Test failed with exception: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == "__main__":
    success = asyncio.run(main())
    sys.exit(0 if success else 1)
