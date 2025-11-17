#!/usr/bin/env python3
"""
Verify merkle proofs via Mint API
Usage: 
  python verify_merkle.py --proof-type token --token-id <token-id>
  python verify_merkle.py --proof-type ingot --ingot-id <ingot-id>
  python verify_merkle.py --proof-type unit --unit-id <unit-id>
"""

import requests
import argparse
import json

MINT_API_URL = "http://localhost:8084"

def verify_merkle_proof(proof_type, item_id):
    """Verify merkle proof for token/ingot/unit"""
    endpoints = {
        "token": f"/verify/merkle/token/{item_id}",
        "ingot": f"/verify/merkle/ingot/{item_id}",
        "unit": f"/verify/merkle/unit/{item_id}"
    }
    
    url = f"{MINT_API_URL}{endpoints[proof_type]}"
    
    try:
        response = requests.get(url, timeout=5)
        response.raise_for_status()
        data = response.json()
        
        print(f"\n{'='*60}")
        print(f"Merkle Proof Verification: {proof_type.upper()}")
        print(f"ID: {item_id}")
        print(f"{'='*60}\n")
        
        if data.get('valid'):
            print(f"✅ VALID MERKLE PROOF")
        else:
            print(f"❌ INVALID MERKLE PROOF")
            print(f"Error: {data.get('error', 'Unknown error')}")
        
        print(f"\nProof Details:")
        print(json.dumps(data, indent=2))
        
        return data.get('valid', False)
    except requests.exceptions.RequestException as e:
        print(f"❌ Verification failed: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Verify merkle proofs")
    parser.add_argument("--proof-type", type=str, required=True, 
                       choices=["token", "ingot", "unit"],
                       help="Type of proof to verify")
    parser.add_argument("--token-id", type=str, help="Token ID (for token proof)")
    parser.add_argument("--ingot-id", type=str, help="Ingot ID (for ingot proof)")
    parser.add_argument("--unit-id", type=str, help="Unit ID (for unit proof)")
    args = parser.parse_args()
    
    # Get the appropriate ID
    item_id = None
    if args.proof_type == "token":
        item_id = args.token_id
    elif args.proof_type == "ingot":
        item_id = args.ingot_id
    elif args.proof_type == "unit":
        item_id = args.unit_id
    
    if not item_id:
        print(f"❌ Must provide --{args.proof_type}-id")
        return
    
    verify_merkle_proof(args.proof_type, item_id)

if __name__ == "__main__":
    main()
