#!/usr/bin/env python3
"""
Stake RoboTorq for a contract via HTTP API
Usage: python stake.py --amount 0.05 [--contract-id CONTRACT]
"""

import requests
import argparse

DIGGER_URL = "http://localhost:3030"

def stake_robotorq(amount, contract_id=None):
    """Stake RoboTorq"""
    url = f"{DIGGER_URL}/contracts/stake"
    payload = {"amount": amount}
    
    if contract_id:
        payload["contract_id"] = contract_id
    
    try:
        response = requests.post(url, json=payload, timeout=5)
        response.raise_for_status()
        print(f"✅ Staked {amount} RT" + (f" for {contract_id}" if contract_id else ""))
        return True
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to stake: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Stake RoboTorq")
    parser.add_argument("--amount", type=float, default=0.05, help="Amount to stake (default: 0.05)")
    parser.add_argument("--contract-id", type=str, help="Contract ID (optional)")
    args = parser.parse_args()
    
    print(f"\n{'='*60}")
    print(f"Staking {args.amount} RT")
    print(f"{'='*60}\n")
    
    stake_robotorq(args.amount, args.contract_id)

if __name__ == "__main__":
    main()
