#!/usr/bin/env python3
"""
Create Digger contracts via HTTP API
Usage: python create_contract.py [--count N] [--duration SECONDS]
"""

import requests
import argparse
import time
from datetime import datetime

DIGGER_URL = "http://localhost:3030"

def create_contract(contract_id=None, duration=3):
    """Create a single contract"""
    if not contract_id:
        timestamp = int(time.time())
        contract_id = f"contract-{timestamp}"
    
    url = f"{DIGGER_URL}/contracts/create"
    payload = {
        "contract_id": contract_id,
        "duration": duration
    }
    
    try:
        response = requests.post(url, json=payload, timeout=5)
        response.raise_for_status()
        print(f"✅ Created contract: {contract_id} ({duration}s)")
        return contract_id
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to create contract: {e}")
        return None

def main():
    parser = argparse.ArgumentParser(description="Create Digger contracts")
    parser.add_argument("--count", type=int, default=1, help="Number of contracts to create")
    parser.add_argument("--duration", type=int, default=3, help="Contract duration in seconds")
    parser.add_argument("--contract-id", type=str, help="Specific contract ID (only for count=1)")
    args = parser.parse_args()
    
    print(f"\n{'='*60}")
    print(f"Creating {args.count} contract(s) - {args.duration}s each")
    print(f"{'='*60}\n")
    
    contracts = []
    timestamp = int(time.time())
    
    for i in range(args.count):
        if args.count == 1 and args.contract_id:
            contract_id = args.contract_id
        else:
            contract_id = f"contract-{timestamp}-{i+1:03d}"
        
        result = create_contract(contract_id, args.duration)
        if result:
            contracts.append(result)
        time.sleep(0.1)  # Small delay between creations
    
    print(f"\n✨ Created {len(contracts)}/{args.count} contracts")
    if contracts:
        print("\nContract IDs:")
        for contract_id in contracts:
            print(f"  - {contract_id}")

if __name__ == "__main__":
    main()
