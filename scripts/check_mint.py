#!/usr/bin/env python3
"""
Check Mint verification API health
Usage: python check_mint.py
"""

import requests
import json

MINT_API_URL = "http://localhost:8084"

def check_health():
    """Check Mint API health"""
    try:
        response = requests.get(f"{MINT_API_URL}/health", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        print(f"\n{'='*60}")
        print(f"Mint Verification API - Health Check")
        print(f"{'='*60}\n")
        
        print(f"Status: {data.get('status', 'unknown')}")
        print(f"Cache Size: {data.get('cache_size', 0)} Phase3 units")
        print(f"Uptime: {data.get('uptime', 'unknown')}")
        
        return True
    except requests.exceptions.RequestException as e:
        print(f"❌ Mint API unreachable: {e}")
        return False

if __name__ == "__main__":
    check_health()
