#!/usr/bin/env python3
"""
Verify Phase3 unit signature via Mint API
Usage: python verify_signature.py --unit-id <unit-id>
"""

import requests
import argparse
import json

MINT_API_URL = "http://localhost:8080"

def verify_signature(unit_id):
    """Verify a Phase3 unit's signature"""
    url = f"{MINT_API_URL}/verify/signature/{unit_id}"
    
    try:
        response = requests.get(url, timeout=5)
        response.raise_for_status()
        data = response.json()
        
        print(f"\n{'='*60}")
        print(f"Signature Verification: {unit_id}")
        print(f"{'='*60}\n")
        
        if data.get('valid'):
            print(f"✅ VALID SIGNATURE")
        else:
            print(f"❌ INVALID SIGNATURE")
            print(f"Error: {data.get('error', 'Unknown error')}")
        
        print(f"\nDetails:")
        print(json.dumps(data, indent=2))
        
        return data.get('valid', False)
    except requests.exceptions.RequestException as e:
        print(f"❌ Verification failed: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Verify Phase3 unit signature")
    parser.add_argument("--unit-id", type=str, required=True, help="Phase3 unit ID")
    args = parser.parse_args()
    
    verify_signature(args.unit_id)

if __name__ == "__main__":
    main()
