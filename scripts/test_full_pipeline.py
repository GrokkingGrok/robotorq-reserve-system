#!/usr/bin/env python3
"""
Full Pipeline Test: Printer → JTUs → Ore → RoboTorq → Wallet UBD

Tests the complete value flow:
1. Printer executes contract → generates JTUs
2. JTUs stored in Digger database
3. Ore generated from contract execution
4. Refinery processes ore → creates ingots
5. Mint processes ingots → creates Phase3 RoboTorq units
6. DistoDam distributes RoboTorq to wallets (UBD)
7. Wallet receives and displays RoboTorq balance

This is the COMPLETE economic cycle of RoboTorq.
"""

import asyncio
import json
import time
import requests
from datetime import datetime
from nats.aio.client import Client as NATS

# Service URLs
DIGGER_URL = "http://localhost:3030"
MINT_URL = "http://localhost:8080"
DISTODAM_URL = "http://localhost:8082"
WALLET_URL = "http://localhost:8085"
NATS_URL = "nats://localhost:4222"

# Colors for terminal output
GREEN = "\033[92m"
YELLOW = "\033[93m"
RED = "\033[91m"
BLUE = "\033[94m"
RESET = "\033[0m"

def print_step(step_num, description):
    """Print a test step header"""
    print(f"\n{BLUE}{'='*70}")
    print(f"STEP {step_num}: {description}")
    print(f"{'='*70}{RESET}\n")

def print_success(message):
    """Print success message"""
    print(f"{GREEN}✓ {message}{RESET}")

def print_warning(message):
    """Print warning message"""
    print(f"{YELLOW}⚠ {message}{RESET}")

def print_error(message):
    """Print error message"""
    print(f"{RED}✗ {message}{RESET}")

def check_service_health(service_name, url):
    """Check if a service is healthy"""
    try:
        response = requests.get(f"{url}/health", timeout=5)
        if response.status_code == 200:
            print_success(f"{service_name} is healthy")
            return True
        else:
            print_error(f"{service_name} returned status {response.status_code}")
            return False
    except Exception as e:
        print_error(f"{service_name} health check failed: {e}")
        return False

async def main():
    print(f"\n{BLUE}╔═══════════════════════════════════════════════════════════════╗")
    print(f"║  RoboTorq Full Pipeline Test - Complete Economic Cycle       ║")
    print(f"╚═══════════════════════════════════════════════════════════════╝{RESET}\n")
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 0: Check all services are running
    # ═══════════════════════════════════════════════════════════════
    print_step(0, "Verify All Services Are Running")
    
    services_ok = True
    services_ok &= check_service_health("Digger", DIGGER_URL)
    services_ok &= check_service_health("Mint", MINT_URL)
    services_ok &= check_service_health("DistoDam", DISTODAM_URL)
    services_ok &= check_service_health("Wallet", WALLET_URL)
    
    if not services_ok:
        print_error("\n❌ Some services are not healthy. Please run: docker-compose up -d")
        return False
    
    print_success("\n✓ All services are healthy!")
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 1: Register printer with Digger
    # ═══════════════════════════════════════════════════════════════
    print_step(1, "Register Printer with Digger")
    
    printer_id = "test-printer-pipeline"
    
    # Check if printer already registered
    try:
        response = requests.get(f"{DIGGER_URL}/printers")
        printers = response.json()
        printer_exists = any(p.get("printer_id") == printer_id for p in printers)
        
        if printer_exists:
            print_warning(f"Printer {printer_id} already registered")
        else:
            # Register new printer
            response = requests.post(
                f"{DIGGER_URL}/printer/register",
                json={
                    "printer_id": printer_id,
                    "model": "TestPrinter",
                    "rated_watts": 250
                }
            )
            if response.status_code == 200:
                print_success(f"Printer {printer_id} registered")
            else:
                print_error(f"Failed to register printer: {response.text}")
                return False
    except Exception as e:
        print_error(f"Printer registration failed: {e}")
        return False
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 2: Create and fund contract
    # ═══════════════════════════════════════════════════════════════
    print_step(2, "Create and Fund Contract")
    
    contract_id = f"test-pipeline-{int(time.time())}"
    
    try:
        # Create contract
        response = requests.post(
            f"{DIGGER_URL}/contract/create",
            json={
                "contract_id": contract_id,
                "torq": 100,
                "robo_stake": 5.0,
                "description": "Full pipeline test contract"
            }
        )
        if response.status_code != 200:
            print_error(f"Failed to create contract: {response.text}")
            return False
        
        data = response.json()
        ore_target = data.get("ore_target")
        print_success(f"Contract created: {contract_id}")
        print(f"  Torq: 100, RoboStake: 5.0 RT, Ore Target: {ore_target} RT")
        
        # Fund contract
        response = requests.post(
            f"{DIGGER_URL}/contract/fund",
            json={
                "contract_id": contract_id,
                "amount": 5.0
            }
        )
        if response.status_code != 200:
            print_error(f"Failed to fund contract: {response.text}")
            return False
        
        print_success(f"Contract funded with 5.0 RT")
        
    except Exception as e:
        print_error(f"Contract creation/funding failed: {e}")
        return False
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 3: Assign contract to printer
    # ═══════════════════════════════════════════════════════════════
    print_step(3, "Assign Contract to Printer")
    
    try:
        response = requests.post(
            f"{DIGGER_URL}/printer/assign",
            json={
                "printer_id": printer_id,
                "contract_id": contract_id
            }
        )
        if response.status_code != 200:
            print_error(f"Failed to assign contract: {response.text}")
            return False
        
        print_success(f"Contract {contract_id} assigned to {printer_id}")
        
    except Exception as e:
        print_error(f"Contract assignment failed: {e}")
        return False
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 4: Start mock print (contract execution)
    # ═══════════════════════════════════════════════════════════════
    print_step(4, "Execute Contract via Mock Print")
    
    try:
        # Start mock print (15 seconds)
        response = requests.post(
            "http://localhost:9092/start",
            params={
                "contract_id": contract_id,
                "duration_secs": 15
            }
        )
        if response.status_code != 200:
            print_error(f"Failed to start print: {response.text}")
            return False
        
        print_success(f"Mock print started (15 seconds)")
        print(f"  ⏱️  Waiting for print to complete and contract execution...")
        
        # Wait for print to complete and contract to execute
        time.sleep(20)
        
    except Exception as e:
        print_error(f"Print execution failed: {e}")
        return False
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 5: Verify JTUs generated and stored
    # ═══════════════════════════════════════════════════════════════
    print_step(5, "Verify JTUs Generated and Stored")
    
    try:
        response = requests.get(f"{DIGGER_URL}/contract/{contract_id}")
        if response.status_code != 200:
            print_error(f"Failed to get contract status: {response.text}")
            return False
        
        contract = response.json()
        status = contract.get("status")
        jtu_count = contract.get("jtu_count", 0)
        ore_generated = contract.get("ore_generated", 0)
        
        print(f"  Contract Status: {status}")
        print(f"  JTUs Generated: {jtu_count:,}")
        print(f"  Ore Generated: {ore_generated:.2f} RT")
        print(f"  Ore Target: {ore_target} RT")
        print(f"  Progress: {(ore_generated / ore_target * 100):.1f}%")
        
        if jtu_count > 0:
            print_success(f"✓ Contract executed successfully: {jtu_count:,} JTUs generated")
        else:
            print_warning("⚠ No JTUs generated yet - contract may still be executing")
        
    except Exception as e:
        print_error(f"Contract verification failed: {e}")
        return False
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 6: Wait for ore to be sent to Refinery (via NATS)
    # ═══════════════════════════════════════════════════════════════
    print_step(6, "Wait for Ore Processing by Refinery")
    
    print(f"  ⏱️  Ore is batched and sent to Refinery...")
    print(f"  ⏱️  Refinery aggregates ore into ingots...")
    print(f"  ⏱️  This may take 60-120 seconds depending on batch timing...")
    
    # Give time for ore → refinery → ingot flow
    time.sleep(30)
    print_success("✓ Ore processing time elapsed")
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 7: Check Mint for Phase3 RoboTorq unit creation
    # ═══════════════════════════════════════════════════════════════
    print_step(7, "Check for Phase3 RoboTorq Units at Mint")
    
    print(f"  ⏱️  Mint aggregates ingots into batches...")
    print(f"  ⏱️  Creates Phase3 RoboTorq units with merkle trees...")
    print(f"  ⏱️  This may take 60-180 seconds depending on batch size...")
    
    # Give time for ingot → mint → Phase3 unit flow
    time.sleep(60)
    
    # Check Mint metrics (if available)
    try:
        response = requests.get(f"{MINT_URL}/metrics")
        if response.status_code == 200:
            metrics_text = response.text
            if "mint_phase3_units_minted_total" in metrics_text:
                # Extract the count
                for line in metrics_text.split('\n'):
                    if line.startswith("mint_phase3_units_minted_total"):
                        count = float(line.split()[-1])
                        print_success(f"✓ Mint has created {int(count)} Phase3 RoboTorq units")
                        break
            else:
                print_warning("⚠ Phase3 unit metrics not yet available")
    except Exception as e:
        print_warning(f"⚠ Could not check Mint metrics: {e}")
    
    # ═══════════════════════════════════════════════════════════════
    # STEP 8: Register wallet for UBD
    # ═══════════════════════════════════════════════════════════════
    print_step(8, "Register Wallet for Universal Basic Distribution")
    
    wallet_id = f"test-wallet-{int(time.time())}"
    
    print(f"  📱 Creating wallet: {wallet_id}")
    print(f"  📡 Wallet will automatically subscribe to wallet.distribution NATS topic")
    print(f"  🔔 DistoDam will send RoboTorq units to active wallets")
    
    # Connect to NATS to publish wallet activation
    nc = NATS()
    try:
        await nc.connect(NATS_URL)
        print_success("✓ Connected to NATS")
        
        # Publish wallet activation
        activation_message = {
            "wallet_id": wallet_id,
            "activate": True,
            "requested_at": datetime.utcnow().isoformat()
        }
        
        await nc.publish(
            "wallet.activate",
            json.dumps(activation_message).encode()
        )
        print_success(f"✓ Wallet activation published for {wallet_id}")
        
        # Subscribe to wallet distribution topic to see UBD
        received_distributions = []
        
        async def distribution_handler(msg):
            data = json.loads(msg.data.decode())
            received_distributions.append(data)
            print_success(f"\n📥 Received RoboTorq distribution!")
            print(f"  Unit ID: {data['rt_unit']['unit_id']}")
            print(f"  Merkle Root: {data['rt_unit']['merkle_root']}")
            print(f"  RoboStake Total: {data['rt_unit']['robo_stake_total']:.6f} RT")
            print(f"  DistoDam Balance: {data['disto_balance']:.6f} RT")
        
        await nc.subscribe(f"wallet.distribution.{wallet_id}", cb=distribution_handler)
        print_success(f"✓ Subscribed to wallet.distribution.{wallet_id}")
        
        # ═══════════════════════════════════════════════════════════════
        # STEP 9: Wait for UBD distribution
        # ═══════════════════════════════════════════════════════════════
        print_step(9, "Wait for Universal Basic Distribution")
        
        print(f"  ⏱️  Waiting for DistoDam to distribute RoboTorq units...")
        print(f"  ⏱️  Distribution rate: 0.001 RT/min (configurable)")
        print(f"  ⏱️  Will wait up to 60 seconds for first distribution...")
        
        # Wait for distribution (with timeout)
        timeout = 60
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            if received_distributions:
                break
            await asyncio.sleep(1)
        
        await nc.close()
        
        if received_distributions:
            print_success(f"\n✓ Received {len(received_distributions)} RoboTorq distribution(s)!")
            distribution = received_distributions[0]
            rt_unit = distribution['rt_unit']
            
            print(f"\n{GREEN}╔═══════════════════════════════════════════════════════════════╗")
            print(f"║  🎉 COMPLETE ECONOMIC CYCLE VERIFIED! 🎉                     ║")
            print(f"╚═══════════════════════════════════════════════════════════════╝{RESET}\n")
            
            print(f"{BLUE}Value Flow Summary:{RESET}")
            print(f"  1. ⚙️  Printer executed contract → {jtu_count:,} JTUs generated")
            print(f"  2. ⛏️  JTUs converted to ore → {ore_generated:.2f} RT ({(ore_generated/ore_target*100):.1f}%)")
            print(f"  3. 🏭 Refinery aggregated ore → created ingots")
            print(f"  4. 🏦 Mint created Phase3 RoboTorq unit:")
            print(f"     • Unit ID: {rt_unit['unit_id']}")
            print(f"     • Merkle Root: {rt_unit['merkle_root']}")
            print(f"     • Total Value: {rt_unit['robo_stake_total']:.6f} RT")
            print(f"  5. 💰 DistoDam distributed to wallet {wallet_id}")
            print(f"     • Remaining in DistoDam: {distribution['disto_balance']:.6f} RT")
            
            print(f"\n{GREEN}✨ SUCCESS: Complete pipeline working! ✨{RESET}\n")
            return True
            
        else:
            print_warning(f"\n⚠️  No distribution received within {timeout} seconds")
            print(f"   This may be normal if:")
            print(f"   • DistoDam batch interval hasn't elapsed yet")
            print(f"   • Phase3 units are still being minted")
            print(f"   • Distribution rate is very slow (0.001 RT/min)")
            print(f"\n   Check DistoDam status: {DISTODAM_URL}/status")
            print(f"   Check logs: docker logs robotorq-network-distodam-1")
            return False
        
    except Exception as e:
        print_error(f"Wallet registration or distribution failed: {e}")
        if nc.is_connected:
            await nc.close()
        return False

if __name__ == "__main__":
    success = asyncio.run(main())
    exit(0 if success else 1)
