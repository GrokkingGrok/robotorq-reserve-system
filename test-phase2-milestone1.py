#!/usr/bin/env python3
"""
Phase 2 Milestone 1 Test - NATS Subscriber
Tests that Refinery receives and logs hash batches from NATS
"""

import os
import sys
import time
import json
import subprocess
import requests
from pathlib import Path

# Colors for terminal output
class Colors:
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    RESET = '\033[0m'

def print_step(step_num, message, color=Colors.YELLOW):
    print(f"\n{color}[STEP {step_num}] {message}{Colors.RESET}")

def print_success(message):
    print(f"{Colors.GREEN}✓ {message}{Colors.RESET}")

def print_error(message):
    print(f"{Colors.RED}✗ {message}{Colors.RESET}")

def run_command(cmd, cwd=None, background=False, env=None):
    """Run a shell command"""
    if background:
        return subprocess.Popen(
            cmd,
            shell=True,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env or os.environ.copy()
        )
    else:
        result = subprocess.run(
            cmd,
            shell=True,
            cwd=cwd,
            capture_output=True,
            text=True,
            env=env or os.environ.copy()
        )
        return result

def main():
    print(f"{Colors.CYAN}[TEST] Phase 2 Milestone 1: NATS Subscriber{Colors.RESET}\n")
    
    # Get project root
    project_root = Path(__file__).parent
    
    # Step 1: Start NATS
    print_step(1, "Starting NATS...")
    result = run_command("docker-compose up -d nats", cwd=project_root)
    if result.returncode == 0:
        print_success("NATS started")
        time.sleep(2)
    else:
        print_error(f"Failed to start NATS: {result.stderr}")
        return 1
    
    # Step 2: Start Refinery in background
    print_step(2, "Starting Refinery...")
    refinery_dir = project_root / "src" / "refinery"
    refinery_log = refinery_dir / "refinery-output.txt"
    refinery_err = refinery_dir / "refinery-error.txt"
    
    with open(refinery_log, "w") as out, open(refinery_err, "w") as err:
        refinery_proc = subprocess.Popen(
            ["go", "run", "cmd/refinery/main.go"],
            cwd=refinery_dir,
            stdout=out,
            stderr=err
        )
    print_success(f"Refinery started (PID: {refinery_proc.pid})")
    time.sleep(5)
    
    # Step 3: Start Digger in background
    print_step(3, "Starting Digger...")
    digger_dir = project_root / "src" / "digger"
    digger_log = digger_dir / "digger-output.txt"
    digger_err = digger_dir / "digger-error.txt"
    
    digger_env = os.environ.copy()
    digger_env["NATS_URL"] = "nats://localhost:4222"
    digger_env["BATCH_INTERVAL_SEC"] = "5"  # Send hashes every 5 seconds
    
    with open(digger_log, "w") as out, open(digger_err, "w") as err:
        digger_proc = subprocess.Popen(
            ["cargo", "run", "--release"],
            cwd=digger_dir,
            stdout=out,
            stderr=err,
            env=digger_env
        )
    print_success(f"Digger started (PID: {digger_proc.pid})")
    
    # Step 4: Wait for Digger HTTP server to be ready
    print_step(4, "Waiting for Digger HTTP server...")
    digger_ready = False
    for attempt in range(30):  # Wait up to 30 seconds
        time.sleep(1)
        try:
            response = requests.get("http://localhost:9000/health", timeout=1)
            if response.status_code == 200:
                print_success("Digger HTTP server ready")
                digger_ready = True
                break
        except requests.exceptions.RequestException:
            pass  # Server not ready yet
    
    if not digger_ready:
        print_error("Digger HTTP server did not start within 30 seconds")
        cleanup(refinery_proc, digger_proc, project_root)
        return 1
    
    # Step 5: Create test contract
    print_step(5, "Creating test contract...")
    create_data = {
        "contract_id": "milestone1-test",
        "torq": 100.0,
        "robo_stake": 5.0,
        "milestones": 10,
        "power_watts": 100.0
    }
    
    try:
        response = requests.post(
            "http://localhost:9000/contracts/create",
            json=create_data,
            timeout=10
        )
        if response.status_code == 200:
            print_success("Contract created successfully")
            resp_json = response.json()
            print(f"  Ore target: {resp_json.get('ore_target', 'N/A')} RT")
        else:
            print_error(f"Contract creation failed: {response.status_code}")
            print(f"  {response.text}")
            cleanup(refinery_proc, digger_proc, project_root)
            return 1
    except Exception as e:
        print_error(f"Failed to create contract: {e}")
        cleanup(refinery_proc, digger_proc, project_root)
        return 1
    
    # Step 6: Pay stake
    print_step(6, "Paying contract stake...")
    stake_data = {
        "contract_id": "milestone1-test"
    }
    
    try:
        response = requests.post(
            "http://localhost:9000/contracts/stake",
            json=stake_data,
            timeout=10
        )
        if response.status_code == 200:
            print_success("Stake paid successfully")
        else:
            print_error(f"Stake payment failed: {response.status_code}")
            print(f"  {response.text}")
            cleanup(refinery_proc, digger_proc, project_root)
            return 1
    except Exception as e:
        print_error(f"Failed to pay stake: {e}")
        cleanup(refinery_proc, digger_proc, project_root)
        return 1
    
    # Step 7: Execute test contract
    print_step(7, "Executing test contract...")
    contract_data = {
        "contract_id": "milestone1-test",
        "duration_seconds": 3
    }
    
    try:
        response = requests.post(
            "http://localhost:9000/contracts/execute",  # Fixed: correct endpoint
            json=contract_data,
            timeout=10
        )
        if response.status_code == 200:
            print_success("Contract executed successfully")
            print(f"  Response: {json.dumps(response.json(), indent=2)}")
        else:
            print_error(f"Contract execution failed: {response.status_code}")
            print(f"  {response.text}")
            cleanup(refinery_proc, digger_proc, project_root)
            return 1
    except Exception as e:
        print_error(f"Failed to execute contract: {e}")
        cleanup(refinery_proc, digger_proc, project_root)
        return 1
    
    # Step 8: Wait for hash transmission
    print_step(8, "Waiting 20 seconds for hash transmission...")
    print(f"  {Colors.YELLOW}(Hash sender runs every 5 seconds, waiting for 2 cycles){Colors.RESET}")
    time.sleep(20)
    
    # Step 9: Check Refinery logs
    print_step(9, "Checking Refinery logs...")
    
    try:
        with open(refinery_log, "r") as f:
            logs = f.read()
        
        if "received hash batch" in logs:
            print_success("Refinery received hash batch!")
            
            # Extract hash batch details
            for line in logs.split('\n'):
                if "received hash batch" in line:
                    print(f"{Colors.CYAN}  {line}{Colors.RESET}")
            
            # Check for hash count
            hash_count_lines = [l for l in logs.split('\n') if "hash_count" in l]
            if hash_count_lines:
                print(f"\n{Colors.YELLOW}Hash batch details:{Colors.RESET}")
                print(f"{Colors.CYAN}  {hash_count_lines[-1]}{Colors.RESET}")
            
            print(f"\n{Colors.GREEN}[MILESTONE 1] ✅ PASSED - NATS Subscriber Working!{Colors.RESET}")
            success = True
        else:
            print_error("No hash batch received in Refinery logs")
            print(f"\n{Colors.YELLOW}Refinery logs (last 20 lines):{Colors.RESET}")
            for line in logs.split('\n')[-20:]:
                print(f"  {line}")
            
            print(f"\n{Colors.RED}[MILESTONE 1] ❌ FAILED{Colors.RESET}")
            success = False
    except Exception as e:
        print_error(f"Failed to read Refinery logs: {e}")
        success = False
    
    # Cleanup
    cleanup(refinery_proc, digger_proc, project_root)
    
    return 0 if success else 1

def cleanup(refinery_proc, digger_proc, project_root):
    """Stop all services"""
    print(f"\n{Colors.YELLOW}[CLEANUP] Stopping services...{Colors.RESET}")
    
    # Stop Refinery
    if refinery_proc:
        refinery_proc.terminate()
        try:
            refinery_proc.wait(timeout=5)
            print_success("Refinery stopped")
        except subprocess.TimeoutExpired:
            refinery_proc.kill()
            print_success("Refinery killed")
    
    # Stop Digger
    if digger_proc:
        digger_proc.terminate()
        try:
            digger_proc.wait(timeout=5)
            print_success("Digger stopped")
        except subprocess.TimeoutExpired:
            digger_proc.kill()
            print_success("Digger killed")
    
    # Stop Docker containers
    run_command("docker-compose down", cwd=project_root)
    print_success("Docker containers stopped")
    
    print(f"\n{Colors.GREEN}[COMPLETE] Test finished!{Colors.RESET}\n")

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print(f"\n{Colors.YELLOW}Test interrupted by user{Colors.RESET}")
        sys.exit(1)
