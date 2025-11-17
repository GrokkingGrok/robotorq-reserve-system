#!/usr/bin/env python3
"""
Full contract workflow: create + stake + execute
Usage: python run_contract.py [--count N] [--duration SECONDS] [--stake AMOUNT]
"""

import requests
import argparse
import time

DIGGER_URL = "http://localhost:3030"

def run_contract(contract_id, duration=3, stake_amount=0.05, power_watts=1500.0):
    """Run complete contract workflow"""
    print(f"\n▶ Processing contract: {contract_id}")
    
    # Calculate milestones based on duration (1 milestone per second)
    milestones = duration
    
    # Create
    try:
        response = requests.post(f"{DIGGER_URL}/contracts/create", 
                                json={
                                    "contract_id": contract_id,
                                    "torq": 100.0,  # 100:1 ratio
                                    "robo_stake": stake_amount,
                                    "milestones": milestones,
                                    "power_watts": power_watts
                                }, 
                                timeout=5)
        response.raise_for_status()
        print(f"  ✅ Created")
    except Exception as e:
        print(f"  ❌ Create failed: {e}")
        return False
    
    # Stake (already paid during create, but endpoint might validate)
    try:
        response = requests.post(f"{DIGGER_URL}/contracts/stake", 
                                json={"contract_id": contract_id}, 
                                timeout=5)
        response.raise_for_status()
        print(f"  ✅ Staked {stake_amount} RT")
    except Exception as e:
        print(f"  ❌ Stake failed: {e}")
        return False
    
    # Execute
    try:
        response = requests.post(f"{DIGGER_URL}/contracts/execute", 
                                json={
                                    "contract_id": contract_id,
                                    "duration_seconds": duration
                                }, 
                                timeout=duration+30)  # Add buffer to timeout
        response.raise_for_status()
        result = response.json()
        jtus = result.get('jtus_generated', 0)
        print(f"  ✅ Executed: {jtus} JTUs generated")
    except Exception as e:
        print(f"  ❌ Execute failed: {e}")
        return False
    
    return True

def main():
    parser = argparse.ArgumentParser(description="Run complete contract workflow")
    parser.add_argument("--count", type=int, default=1, help="Number of contracts")
    parser.add_argument("--duration", type=int, default=3, help="Contract duration (seconds)")
    parser.add_argument("--stake", type=float, default=0.05, help="Stake amount")
    args = parser.parse_args()
    
    print(f"\n{'='*60}")
    print(f"Running {args.count} Contract(s)")
    print(f"Duration: {args.duration}s | Stake: {args.stake} RT")
    print(f"{'='*60}")
    
    timestamp = int(time.time())
    success = 0
    
    for i in range(args.count):
        contract_id = f"contract-{timestamp}-{i+1:03d}"
        if run_contract(contract_id, args.duration, args.stake):
            success += 1
        time.sleep(0.2)
    
    print(f"\n✨ Successfully ran {success}/{args.count} contracts")

if __name__ == "__main__":
    main()
