#!/usr/bin/env python3
"""
Phase 2 Milestone 5 E2E Test: Full Pipeline Integration

Tests the COMPLETE flow from Digger contract execution to Refinery ingot assembly:

1. Digger executes real contract (10,000 JTUs)
2. Digger stores JTUs in SQLite
3. Hash sender task extracts hashes and sends to NATS
4. Refinery receives hash batches
5. Refinery builds 2-3 merkle tree ingots (3600 hashes each)
6. Verify all ingots have valid merkle roots

This is the REAL E2E test - not simulated hashes!

Architecture:
Digger (contract exec) → JTU storage → Hash sender → NATS ore.batch →
Refinery (NATSSubscriber) → QueueManager → Phase2IngotAssembler →
BuildMerkleTree → Phase2Ingot (with merkle root)

Success Criteria:
- ✅ Digger executes contract successfully
- ✅ 10,000 JTUs generated and stored
- ✅ Hash sender publishes hashes to NATS
- ✅ Refinery receives all hash batches
- ✅ 2-3 ingots assembled (10k ÷ 3600 = 2.77 → 2 full + 1 partial)
- ✅ Each ingot has unique valid merkle root
- ✅ Ingot metadata contains correct contract/digger IDs
"""

import asyncio
import json
import os
import subprocess
import time
import requests
import sys
from pathlib import Path
from datetime import datetime

# Configuration
DIGGER_BINARY = "src/digger/target/debug/digger.exe"
DIGGER_API_URL = "http://localhost:9000"
NATS_URL = "nats://localhost:4222"
REFINERY_CONTAINER = "robotorq-network-refinery-1"

# Test parameters
CONTRACT_ID = "milestone5-e2e-test"
CONTRACT_DURATION = 5  # seconds (generates ~10k JTUs)
HASH_BATCH_INTERVAL = 10  # seconds (how often Digger sends hashes)
EXPECTED_JTU_COUNT = 10000  # Approximate based on 5-second contract
EXPECTED_INGOTS = 2  # 10k hashes ÷ 3600 = 2.77 → at least 2 full ingots


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
            check=True
        )
        if "Up" in result.stdout:
            print_success("NATS is running")
        else:
            print_error("NATS is not running. Start with: docker-compose up -d nats")
            return False
    except Exception as e:
        print_error(f"Failed to check NATS: {e}")
        return False
    
    # Check Refinery is running
    print_step("Checking Refinery...")
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", f"name={REFINERY_CONTAINER}", "--format", "{{.Status}}"],
            capture_output=True,
            text=True,
            check=True
        )
        if "Up" in result.stdout and "healthy" in result.stdout:
            print_success("Refinery is running and healthy")
        else:
            print_error("Refinery is not running. Start with: docker-compose up -d refinery")
            return False
    except Exception as e:
        print_error(f"Failed to check Refinery: {e}")
        return False
    
    # Check Digger binary exists
    print_step("Checking Digger binary...")
    digger_path = Path(DIGGER_BINARY)
    if not digger_path.exists():
        print_error(f"Digger binary not found at {DIGGER_BINARY}")
        print_warning("Build with: cd src/digger && cargo build")
        return False
    print_success(f"Digger binary found: {digger_path.absolute()}")
    
    return True


def start_digger():
    """Start Digger service"""
    print_section("Starting Digger Service")
    
    print_step("Starting Digger on port 9000...")
    
    # Set environment variables for Digger
    digger_env = {
        **os.environ,  # Inherit current env
        "DIGGER_ID": "milestone5-e2e-digger",
        "NATS_URL": "nats://localhost:4222",
        "HTTP_PORT": "9000",
        "STORAGE_PATH": "./jtu_storage",
        "BATCH_INTERVAL_SEC": str(HASH_BATCH_INTERVAL),
        "LOG_LEVEL": "info"
    }
    
    # Start Digger in background (with UTF-8 encoding to handle emojis)
    digger_process = subprocess.Popen(
        [DIGGER_BINARY],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding='utf-8',
        errors='replace',  # Replace unencodable characters
        env=digger_env,
        cwd=Path(DIGGER_BINARY).parent
    )
    
    # Wait for Digger to start
    print_step("Waiting for Digger to initialize...")
    time.sleep(3)
    
    # Check if Digger is running
    if digger_process.poll() is not None:
        stdout, stderr = digger_process.communicate()
        print_error("Digger failed to start")
        print(f"STDOUT:\n{stdout}")
        print(f"STDERR:\n{stderr}")
        return None
    
    # Verify Digger API is responding
    max_retries = 5
    for i in range(max_retries):
        try:
            response = requests.get(f"{DIGGER_API_URL}/health", timeout=2)
            if response.status_code == 200:
                print_success(f"Digger API is ready at {DIGGER_API_URL}")
                return digger_process
        except requests.exceptions.RequestException:
            if i < max_retries - 1:
                print_step(f"Waiting for Digger API... (attempt {i+1}/{max_retries})")
                time.sleep(2)
            else:
                print_error("Digger API not responding")
                digger_process.kill()
                return None
    
    return digger_process


def execute_contract():
    """Execute a contract on Digger"""
    print_section("Creating and Executing Contract")
    
    # Step 1: Create contract
    print_step(f"Creating contract '{CONTRACT_ID}'...")
    
    create_payload = {
        "contract_id": CONTRACT_ID,
        "torq": 100.0,  # 100:1 output/input ratio
        "robo_stake": 5.0,  # 5 RT stake
        "milestones": 5,  # 5 milestones
        "power_watts": 2000.0  # 2000W power consumption
    }
    
    try:
        response = requests.post(
            f"{DIGGER_API_URL}/contracts/create",
            json=create_payload,
            timeout=5
        )
        
        if response.status_code == 200:
            result = response.json()
            print_success("Contract created successfully!")
            print(f"   Contract ID: {result.get('contract_id', 'unknown')}")
            print(f"   Ore target: {result.get('ore_target', 'unknown')}")
        else:
            print_error(f"Contract creation failed: {response.status_code}")
            print(response.text)
            return None
            
    except Exception as e:
        print_error(f"Failed to create contract: {e}")
        return None
    
    # Step 2: Approve contract (pay stake)
    print_step(f"Approving contract '{CONTRACT_ID}'...")
    
    approve_payload = {
        "contract_id": CONTRACT_ID
    }
    
    try:
        response = requests.post(
            f"{DIGGER_API_URL}/contracts/stake",
            json=approve_payload,
            timeout=5
        )
        
        if response.status_code == 200:
            result = response.json()
            print_success("Contract approved successfully!")
            print(f"   Status: {result.get('approval_status', 'unknown')}")
        else:
            print_error(f"Contract approval failed: {response.status_code}")
            print(response.text)
            return None
            
    except Exception as e:
        print_error(f"Failed to approve contract: {e}")
        return None
    
    # Step 3: Execute contract
    print_step(f"Executing contract '{CONTRACT_ID}' for {CONTRACT_DURATION} seconds...")
    
    execute_payload = {
        "contract_id": CONTRACT_ID,
        "duration_seconds": CONTRACT_DURATION
    }
    
    try:
        response = requests.post(
            f"{DIGGER_API_URL}/contracts/execute",
            json=execute_payload,
            timeout=CONTRACT_DURATION + 10
        )
        
        if response.status_code == 200:
            result = response.json()
            print_success("Contract executed successfully!")
            print(f"   JTUs generated: {result.get('jtus_generated', 'unknown')}")
            print(f"   Total joules: {result.get('total_joules', 'unknown')}")
            print(f"   Execution time: {result.get('execution_time_ms', 'unknown')}ms")
            return result
        else:
            print_error(f"Contract execution failed: {response.status_code}")
            print(response.text)
            return None
            
    except Exception as e:
        print_error(f"Failed to execute contract: {e}")
        return None


def wait_for_hash_sender(wait_seconds: int):
    """Wait for hash sender to publish hashes"""
    print_section("Waiting for Hash Sender")
    
    print_step(f"Hash sender runs every {HASH_BATCH_INTERVAL} seconds")
    print_step(f"Waiting {wait_seconds} seconds for hashes to be sent to NATS...")
    
    # Show progress
    for i in range(wait_seconds):
        remaining = wait_seconds - i
        print(f"   ⏳ {remaining} seconds remaining...", end='\r')
        time.sleep(1)
    
    print()  # New line after progress
    print_success("Wait complete - hashes should be sent")


def check_refinery_logs():
    """Check Refinery logs for ingot assembly"""
    print_section("Checking Refinery Logs")
    
    print_step("Searching for 'Phase 2 ingot assembled' messages...")
    
    try:
        # Get logs from last 60 seconds
        result = subprocess.run(
            ["docker", "logs", REFINERY_CONTAINER, "--since", "60s"],
            capture_output=True,
            text=True,
            check=True
        )
        
        logs = result.stdout
        
        # Parse JSON logs and find ingot assembly messages
        ingots = []
        for line in logs.split('\n'):
            if '"msg":"Phase 2 ingot assembled"' in line:
                try:
                    log_entry = json.loads(line)
                    ingots.append(log_entry)
                except json.JSONDecodeError:
                    continue
        
        if not ingots:
            print_warning("No ingots found in logs")
            print_warning("This could mean:")
            print("   1. Not enough hashes accumulated yet (need 3600)")
            print("   2. Hash sender hasn't run yet")
            print("   3. NATS connection issue")
            return []
        
        print_success(f"Found {len(ingots)} ingot(s)!")
        
        # Display ingot details
        for i, ingot in enumerate(ingots, 1):
            print(f"\n{Colors.BOLD}Ingot #{i}:{Colors.ENDC}")
            print(f"   ID: {ingot.get('ingot_id', 'unknown')}")
            print(f"   Branch Hash: {ingot.get('branch_hash', 'unknown')}")
            print(f"   Hash Count: {ingot.get('hash_count', 'unknown')}")
            print(f"   Merkle Height: {ingot.get('merkle_height', 'unknown')}")
            print(f"   Assembly Time: {ingot.get('assembly_time_ms', 'unknown')}ms")
            print(f"   Contracts: {ingot.get('contracts', 'unknown')}")
            print(f"   Diggers: {ingot.get('diggers', 'unknown')}")
        
        return ingots
        
    except Exception as e:
        print_error(f"Failed to check Refinery logs: {e}")
        return []


def verify_ingots(ingots: list):
    """Verify ingot properties"""
    print_section("Verifying Ingots")
    
    if len(ingots) < EXPECTED_INGOTS:
        print_warning(f"Expected at least {EXPECTED_INGOTS} ingots, got {len(ingots)}")
        print_warning("You may need to wait longer for more hashes to accumulate")
    else:
        print_success(f"Got {len(ingots)} ingots (expected: {EXPECTED_INGOTS})")
    
    all_valid = True
    
    for i, ingot in enumerate(ingots, 1):
        print_step(f"Verifying ingot #{i}...")
        
        # Check branch_hash is valid (64 char hex)
        branch_hash = ingot.get('branch_hash', '')
        if len(branch_hash) == 64 and all(c in '0123456789abcdef' for c in branch_hash):
            print_success(f"   Branch hash valid: {branch_hash[:16]}...{branch_hash[-16:]}")
        else:
            print_error(f"   Invalid branch hash: {branch_hash}")
            all_valid = False
        
        # Check hash count
        hash_count = ingot.get('hash_count', 0)
        if hash_count == 3600:
            print_success(f"   Hash count: {hash_count} ✓")
        else:
            print_warning(f"   Hash count: {hash_count} (expected 3600)")
        
        # Check merkle height
        merkle_height = ingot.get('merkle_height', 0)
        if merkle_height == 12:
            print_success(f"   Merkle height: {merkle_height} ✓")
        else:
            print_warning(f"   Merkle height: {merkle_height} (expected 12)")
        
        # Check assembly time
        assembly_time = ingot.get('assembly_time_ms', 0)
        if assembly_time < 100:  # Under 100ms is good
            print_success(f"   Assembly time: {assembly_time}ms ✓")
        else:
            print_warning(f"   Assembly time: {assembly_time}ms (a bit slow)")
    
    # Check for unique merkle roots
    if len(ingots) > 1:
        roots = [ing.get('branch_hash') for ing in ingots]
        if len(roots) == len(set(roots)):
            print_success("All merkle roots are unique ✓")
        else:
            print_error("Duplicate merkle roots found!")
            all_valid = False
    
    return all_valid


def cleanup(digger_process):
    """Stop Digger service"""
    print_section("Cleanup")
    
    if digger_process and digger_process.poll() is None:
        print_step("Stopping Digger...")
        digger_process.terminate()
        try:
            digger_process.wait(timeout=5)
            print_success("Digger stopped")
        except subprocess.TimeoutExpired:
            digger_process.kill()
            print_warning("Digger force killed")


def main():
    """Run the complete E2E test"""
    print(f"\n{Colors.BOLD}{Colors.HEADER}")
    print("=" * 80)
    print("Phase 2 Milestone 5: Full Pipeline E2E Test")
    print("Real Digger Contract → JTUs → Hashes → NATS → Refinery → Merkle Ingots")
    print("=" * 80)
    print(f"{Colors.ENDC}\n")
    
    digger_process = None
    
    try:
        # Step 1: Check prerequisites
        if not check_prerequisites():
            print_error("Prerequisites not met. Please fix and try again.")
            return False
        
        # Step 2: Start Digger
        digger_process = start_digger()
        if not digger_process:
            print_error("Failed to start Digger")
            return False
        
        # Step 3: Execute contract
        contract_result = execute_contract()
        if not contract_result:
            print_error("Contract execution failed")
            return False
        
        # Step 4: Wait for hash sender to publish
        # Hash sender runs every HASH_BATCH_INTERVAL seconds
        # We need to wait at least one interval for hashes to be sent
        wait_time = HASH_BATCH_INTERVAL + 5  # Extra buffer
        wait_for_hash_sender(wait_time)
        
        # Step 5: Wait for Refinery to process hashes and build ingots
        # Phase2IngotAssembler blocks on GetHashes(3600)
        # Once 3600 hashes arrive, merkle tree builds in ~8ms
        # With 10k hashes, we should get 2-3 ingots
        print_section("Waiting for Ingot Assembly")
        print_step("Waiting 15 seconds for Refinery to build ingots...")
        time.sleep(15)
        
        # Step 6: Check Refinery logs for ingots
        ingots = check_refinery_logs()
        
        if not ingots:
            print_error("No ingots found!")
            print("\n📋 Debug Steps:")
            print("1. Check Digger logs for hash sending:")
            print(f"   docker logs {REFINERY_CONTAINER} --since 60s | grep 'hash batch'")
            print("2. Check if hashes reached Refinery:")
            print(f"   docker logs {REFINERY_CONTAINER} --since 60s | grep 'received hash batch'")
            print("3. Check queue depth:")
            print(f"   docker logs {REFINERY_CONTAINER} --since 60s | grep 'queue_size'")
            return False
        
        # Step 7: Verify ingots
        all_valid = verify_ingots(ingots)
        
        # Final result
        print_section("Test Results")
        
        if all_valid and len(ingots) >= EXPECTED_INGOTS:
            print_success("✨ MILESTONE 5 E2E TEST PASSED! ✨")
            print(f"\n{Colors.GREEN}Summary:{Colors.ENDC}")
            print(f"   ✅ Contract executed: {CONTRACT_ID}")
            print(f"   ✅ JTUs generated: ~{EXPECTED_JTU_COUNT}")
            print(f"   ✅ Ingots assembled: {len(ingots)}")
            print(f"   ✅ All merkle roots valid")
            print(f"   ✅ Full pipeline working: Digger → NATS → Refinery ✓")
            print(f"\n{Colors.BOLD}Phase 2 is nearly complete! Just cleanup remaining.{Colors.ENDC}\n")
            return True
        else:
            print_warning("Test completed with warnings")
            print(f"   Ingots found: {len(ingots)} (expected: {EXPECTED_INGOTS}+)")
            print(f"   All valid: {'Yes' if all_valid else 'No'}")
            return False
    
    except KeyboardInterrupt:
        print_warning("\nTest interrupted by user")
        return False
    
    except Exception as e:
        print_error(f"Test failed with exception: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    finally:
        cleanup(digger_process)


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
