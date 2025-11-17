#!/usr/bin/env python3
"""
Get Mint's SPHINCS+ public key
Usage: python get_mint_pubkey.py
"""

import requests
import base64

MINT_API_URL = "http://localhost:8084"

def get_public_key():
    """Get Mint's public key"""
    try:
        response = requests.get(f"{MINT_API_URL}/public-key", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        print(f"\n{'='*60}")
        print(f"Mint SPHINCS+ Public Key")
        print(f"{'='*60}\n")
        
        print(f"Algorithm: {data.get('algorithm', 'unknown')}")
        print(f"Key Size: {len(data.get('public_key', ''))} bytes")
        print(f"\nPublic Key (base64):")
        print(f"{data.get('public_key', 'N/A')[:80]}...")
        
        # Decode and show hex
        if data.get('public_key'):
            pubkey_bytes = base64.b64decode(data['public_key'])
            print(f"\nPublic Key (hex, first 64 chars):")
            print(f"{pubkey_bytes.hex()[:64]}...")
        
        return data.get('public_key')
    except requests.exceptions.RequestException as e:
        print(f"❌ Failed to get public key: {e}")
        return None

if __name__ == "__main__":
    get_public_key()
