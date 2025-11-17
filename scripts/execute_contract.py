#!/usr/bin/env python3
"""
Execute a Digger contract via HTTP API
Usage: python execute_contract.py --contract-id CONTRACT
"""

import requests
import argparse

DIGGER_URL = "http://localhost:9000"

def execute_contract(contract_id):
    """Execute a contract"""
    url = f"{DIGGER_URL}/execute"
    payload = {"contract_id": contract_id}
    
    try:
        response = requests.post(url, json=payload, timeout=5)
        response.raise_for_status()
        print(f"✅ Executing contract: {contract_id}")
        return True
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to execute contract: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Execute Digger contract")
    parser.add_argument("--contract-id", type=str, required=True, help="Contract ID to execute")
    args = parser.parse_args()
    
    print(f"\n{'='*60}")
    print(f"Executing Contract: {args.contract_id}")
    print(f"{'='*60}\n")
    
    execute_contract(args.contract_id)

if __name__ == "__main__":
    main()
