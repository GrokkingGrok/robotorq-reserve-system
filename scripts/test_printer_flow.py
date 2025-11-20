#!/usr/bin/env python3
"""
Test Printer Registration → Contract Assignment → Print Flow

What this does:
1. Verifies printer registered with Digger
2. Creates a test contract
3. Assigns contract to printer
4. Triggers mock print start
5. Completes mock print
6. Watches for ore generation

Usage:
    python scripts/test_printer_flow.py
"""

import requests
import time
import json

# Colors
GREEN = '\033[92m'
YELLOW = '\033[93m'
RED = '\033[91m'
BLUE = '\033[94m'
RESET = '\033[0m'

def print_success(msg):
    print(f"{GREEN}✅ {msg}{RESET}")

def print_info(msg):
    print(f"{BLUE}ℹ️  {msg}{RESET}")

def print_error(msg):
    print(f"{RED}❌ {msg}{RESET}")

def print_step(num, msg):
    print(f"\n{YELLOW}Step {num}: {msg}{RESET}")

DIGGER_URL = "http://localhost:3030"
PRINTER_MOCK_URL = "http://localhost:9092"

def check_printer_registered():
    """Check if printer is registered with Digger"""
    print_step(1, "Checking if printer is registered...")
    
    try:
        response = requests.get(f"{DIGGER_URL}/printers")
        if response.status_code == 200:
            printers = response.json()
            if printers and len(printers) > 0:
                print_success(f"Found {len(printers)} registered printer(s)")
                for printer in printers:
                    print(f"  - {printer.get('printer_id', 'unknown')}")
                return printers[0].get('printer_id')
            else:
                print_error("No printers registered yet")
                print_info("Wait a few seconds and check printer logs:")
                print_info("  docker logs robotorq-network-printer-1")
                return None
        else:
            print_error(f"Failed to get printers: HTTP {response.status_code}")
            return None
    except Exception as e:
        print_error(f"Could not connect to Digger: {e}")
        print_info("Is Digger running? docker-compose ps digger")
        return None

def create_test_contract():
    """Create a test contract on Digger"""
    print_step(2, "Creating test contract...")
    
    contract_data = {
        "contract_id": "test-print-contract-001",
        "description": "Test print job for mock printer",
        "torq": 1800000.0,  # 30 minutes at 1000W = 1.8MJ
        "robo_stake": 0.05,
    }
    
    try:
        response = requests.post(f"{DIGGER_URL}/contracts", json=contract_data)
        if response.status_code in [200, 201]:
            print_success("Contract created")
            print(f"  Contract ID: {contract_data['contract_id']}")
            return contract_data['contract_id']
        else:
            print_error(f"Failed to create contract: HTTP {response.status_code}")
            print(response.text)
            return None
    except Exception as e:
        print_error(f"Could not create contract: {e}")
        return None

def assign_contract_to_printer(contract_id, printer_id):
    """Assign contract to printer"""
    print_step(3, "Assigning contract to printer...")
    
    try:
        response = requests.post(
            f"{DIGGER_URL}/printers/{printer_id}/assign",
            json={"contract_id": contract_id}
        )
        if response.status_code == 200:
            print_success(f"Contract assigned to {printer_id}")
            return True
        else:
            print_error(f"Failed to assign contract: HTTP {response.status_code}")
            print(response.text)
            return False
    except Exception as e:
        print_error(f"Could not assign contract: {e}")
        return False

def start_mock_print(contract_id):
    """Trigger mock print start via printer's mock API"""
    print_step(4, "Starting mock print job...")
    
    try:
        response = requests.post(
            f"{PRINTER_MOCK_URL}/start",
            json={"contract_id": contract_id}
        )
        if response.status_code == 200:
            print_success("Mock print started")
            return True
        else:
            print_error(f"Failed to start print: HTTP {response.status_code}")
            print(response.text)
            return False
    except Exception as e:
        print_error(f"Could not start mock print: {e}")
        print_info("Is MOCK_MODE=true in docker-compose? Check printer service")
        return False

def complete_mock_print(contract_id):
    """Trigger mock print completion"""
    print_step(5, "Completing mock print job...")
    
    completion_data = {
        "contract_id": contract_id,
        "capacity_watt_hours": 500.0,  # 0.5 kWh print capacity
    }
    
    try:
        response = requests.post(
            f"{PRINTER_MOCK_URL}/complete",
            json=completion_data
        )
        if response.status_code == 200:
            print_success("Mock print completed")
            print(f"  Capacity: {completion_data['capacity_watt_hours']} Wh")
            return True
        else:
            print_error(f"Failed to complete print: HTTP {response.status_code}")
            print(response.text)
            return False
    except Exception as e:
        print_error(f"Could not complete mock print: {e}")
        return False

def watch_for_ore():
    """Watch Digger logs for ore generation"""
    print_step(6, "Watching for ore generation...")
    print_info("Checking Digger logs in 5 seconds...")
    
    time.sleep(5)
    
    import subprocess
    result = subprocess.run(
        ["docker", "logs", "robotorq-network-digger-1", "--tail", "50"],
        capture_output=True,
        text=True
    )
    
    if "ore" in result.stdout.lower():
        print_success("Ore generation detected in Digger logs!")
        print("\n--- Recent Digger Logs ---")
        for line in result.stdout.split('\n')[-10:]:
            if line.strip():
                print(f"  {line}")
        return True
    else:
        print_error("No ore generation found yet")
        print_info("Check logs manually: docker logs robotorq-network-digger-1")
        return False

def main():
    print()
    print("="*60)
    print(f"{BLUE}🖨️  Test Printer → Contract → Ore Flow{RESET}")
    print("="*60)
    
    # Step 1: Check printer registration
    printer_id = check_printer_registered()
    if not printer_id:
        print()
        print_error("Cannot continue without registered printer")
        print_info("Wait for printer to register, then try again")
        return
    
    # Step 2: Create contract
    contract_id = create_test_contract()
    if not contract_id:
        return
    
    # Step 3: Assign contract
    if not assign_contract_to_printer(contract_id, printer_id):
        return
    
    print_info("Waiting 2 seconds for NATS propagation...")
    time.sleep(2)
    
    # Step 4: Start mock print
    if not start_mock_print(contract_id):
        return
    
    print_info("Waiting 3 seconds for print 'execution'...")
    time.sleep(3)
    
    # Step 5: Complete mock print
    if not complete_mock_print(contract_id):
        return
    
    # Step 6: Watch for ore
    watch_for_ore()
    
    print()
    print("="*60)
    print(f"{GREEN}✨ Test Complete!{RESET}")
    print("="*60)
    print()
    print(f"{YELLOW}Monitor the full pipeline:{RESET}")
    print("  Printer logs:  docker logs -f robotorq-network-printer-1")
    print("  Digger logs:   docker logs -f robotorq-network-digger-1")
    print("  Refinery logs: docker logs -f robotorq-network-refinery-1")
    print("  Mint logs:     docker logs -f robotorq-network-mint-1")
    print()
    print(f"{YELLOW}Check metrics:{RESET}")
    print("  Printer:  curl http://localhost:9094/metrics")
    print("  Digger:   curl http://localhost:9090/metrics")
    print()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
        print_error("Interrupted by user")
    except Exception as e:
        print()
        print_error(f"Unexpected error: {e}")
        import traceback
        traceback.print_exc()
