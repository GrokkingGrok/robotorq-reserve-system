#!/usr/bin/env python3
"""
Test Mock Printer with Milestone Reporting
Assumes Digger is running on localhost:9000
"""

import requests
import subprocess
import sys
import time

# Color helpers
class Colors:
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    CYAN = '\033[96m'
    RESET = '\033[0m'

def print_success(msg: str):
    print(f"{Colors.GREEN}✅ {msg}{Colors.RESET}")

def print_info(msg: str):
    print(f"{Colors.CYAN}   {msg}{Colors.RESET}")

def print_warning(msg: str):
    print(f"{Colors.YELLOW}   {msg}{Colors.RESET}")

def print_error(msg: str):
    print(f"{Colors.RED}❌ {msg}{Colors.RESET}")

def print_section(msg: str):
    print(f"\n{Colors.GREEN}=== {msg} ==={Colors.RESET}")

def main():
    print_section("Mock Printer Test - Milestone Reporting")
    
    # Step 1: Check if Digger is running
    print(f"\n{Colors.YELLOW}[1] Checking Digger availability...{Colors.RESET}")
    try:
        response = requests.get("http://localhost:9000/health", timeout=5)
        if response.status_code == 200:
            health = response.json()
            print_success(f"Digger is running: {health['service']} v{health['version']}")
        else:
            print_error("Digger health check failed")
            print_warning("Start Digger first: cd src/digger && cargo run")
            sys.exit(1)
    except Exception as e:
        print_error("Digger not running on port 9000")
        print_warning("Start Digger first: cd src/digger && cargo run")
        sys.exit(1)
    
    # Step 2: Run mock printer
    print(f"\n{Colors.YELLOW}[2] Starting mock printer...{Colors.RESET}")
    print_info("Duration: 20 seconds")
    print_info("Milestones: 4")
    
    result = subprocess.run([
        "python", "scripts/mock_printer.py",
        "--duration", "20",
        "--milestones", "4",
        "--torq", "100",
        "--robo-stake", "5"
    ])
    
    if result.returncode == 0:
        print_success("\nMock printer test PASSED!")
    else:
        print_error("\nMock printer test FAILED!")
        sys.exit(1)
    
    print_section("Test Complete")

if __name__ == "__main__":
    main()
