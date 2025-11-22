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

def register_printer():
    """Trigger printer registration with Digger"""
    print_step(1, "Registering printer with Digger...")
    
    try:
        # First check if already registered
        response = requests.get(f"{DIGGER_URL}/printers")
        if response.status_code == 200:
            printers = response.json()
            if printers and len(printers) > 0:
                print_success(f"Printer already registered: {printers[0].get('printer_id')}")
                return printers[0].get('printer_id')
        
        # Not registered - delete cert file to force fresh registration
        print_info("Printer not in Digger registry - forcing fresh registration...")
        import subprocess
        import time
        
        # Delete certificate and restart printer
        subprocess.run(["powershell", "-Command", 
                       "Remove-Item src\\printer\\data\\*.json -Force; docker-compose restart printer"],
                      shell=True, check=False)
        
        print_info("Waiting 10 seconds for printer to register...")
        time.sleep(10)
        
        # Check again
        response = requests.get(f"{DIGGER_URL}/printers")
        if response.status_code == 200:
            printers = response.json()
            if printers and len(printers) > 0:
                print_success(f"Printer registered: {printers[0].get('printer_id')}")
                return printers[0].get('printer_id')
        
        print_error("Printer failed to register")
        print_info("Check printer logs: docker logs robotorq-network-printer-1")
        return None
        
    except Exception as e:
        print_error(f"Failed to register printer: {e}")
        return None

def check_printer_registered():
    """Check if printer is registered with Digger"""
    print_step(2, "Verifying printer registration...")
    
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
    
    import time
    contract_id = f"test-print-contract-{int(time.time())}"
    
    contract_data = {
        "contract_id": contract_id,
        "torq": 100.0,
        "robo_stake": 5.0,
        "milestones": 10,
        "power_watts": 1000.0,
    }
    
    try:
        response = requests.post(f"{DIGGER_URL}/contracts/create", json=contract_data)
        if response.status_code in [200, 201]:
            print_success("Contract created")
            print(f"  Contract ID: {contract_data['contract_id']}")
            print(f"  Torq: {contract_data['torq']} RT")
            print(f"  RoboStake: {contract_data['robo_stake']} RT")
            return contract_data['contract_id']
        else:
            print_error(f"Failed to create contract: HTTP {response.status_code}")
            print(response.text)
            return None
    except Exception as e:
        print_error(f"Could not create contract: {e}")
        return None

def fund_contract(contract_id, amount):
    """Fund contract from DistoDam (mock)"""
    print_step(3, "Funding contract (mock DistoDam payment)...")
    
    try:
        response = requests.post(
            f"{DIGGER_URL}/contracts/fund",
            json={"contract_id": contract_id, "amount": amount}
        )
        if response.status_code == 200:
            print_success(f"Contract funded with {amount} RT")
            return True
        else:
            print_error(f"Failed to fund contract: HTTP {response.status_code}")
            print(response.text)
            return False
    except Exception as e:
        print_error(f"Could not fund contract: {e}")
        return False

def assign_contract_to_printer(contract_id, printer_id):
    """Assign contract to printer"""
    print_step(4, "Assigning contract to printer...")
    
    try:
        response = requests.post(
            f"{DIGGER_URL}/printer/assign",
            json={"printer_id": printer_id, "contract_id": contract_id}
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
    print_step(5, "Starting mock print job...")
    
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
    print_step(6, "Completing mock print job...")
    
    try:
        response = requests.post(f"{PRINTER_MOCK_URL}/complete")
        if response.status_code == 200:
            print_success("Mock print completed")
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
    print_step(7, "Watching for ore generation...")
    print_info("Checking Digger logs in 5 seconds...")
    
    time.sleep(5)
    
    import subprocess
    result = subprocess.run(
        ["docker", "logs", "robotorq-network-digger-1", "--tail", "50"],
        capture_output=True,
        text=True,
        encoding='utf-8',
        errors='replace'
    )
    
    if result.stdout and "ore" in result.stdout.lower():
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
    
    # Step 1: Register printer (or verify it's registered)
    printer_id = register_printer()
    if not printer_id:
        print()
        print_error("Cannot continue without registered printer")
        print_info("Check printer logs: docker logs robotorq-network-printer-1")
        return
    
    # Step 2: Create contract
    contract_id = create_test_contract()
    if not contract_id:
        return
    
    # Step 3: Fund contract (mock DistoDam)
    if not fund_contract(contract_id, 5.0):
        return
    
    # Step 4: Assign contract
    if not assign_contract_to_printer(contract_id, printer_id):
        return
    
    print_info("Waiting 2 seconds for NATS propagation...")
    time.sleep(2)
    
    # Step 5: Start mock print
    if not start_mock_print(contract_id):
        return
    
    print_info("Waiting 15 seconds for print 'execution' (heartbeat is 10s)...")
    time.sleep(15)
    
    # Step 6: Complete mock print
    if not complete_mock_print(contract_id):
        return
    
    # Step 7: Watch for ore
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
