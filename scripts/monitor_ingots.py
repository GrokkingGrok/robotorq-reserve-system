#!/usr/bin/env python3
"""
Monitor Mint ingot count via health endpoint
Usage: python monitor_ingots.py [--watch]
"""

import requests
import argparse
import time

MINT_API_URL = "http://localhost:8084"

def get_ingot_count():
    """Get current ingot count from Mint"""
    try:
        response = requests.get(f"{MINT_API_URL}/health", timeout=5)
        response.raise_for_status()
        data = response.json()
        
        # Extract cache size (number of Phase3 units)
        cache_size = data.get('cache_size', 0)
        return cache_size, True
    except requests.exceptions.RequestException:
        return 0, False

def monitor(watch=False, interval=10):
    """Monitor ingot accumulation"""
    print(f"\n{'='*60}")
    print(f"Monitoring Mint Ingot Accumulation")
    if watch:
        print(f"Watching every {interval}s (Ctrl+C to stop)")
    print(f"{'='*60}\n")
    
    if not watch:
        count, success = get_ingot_count()
        if success:
            print(f"Phase3 Units in cache: {count}")
            print(f"Ingots needed for next unit: {1000 - (count * 1000) % 1000}")
        else:
            print("❌ Failed to connect to Mint API")
        return
    
    # Watch mode
    last_count = -1
    try:
        while True:
            count, success = get_ingot_count()
            
            if success:
                if count != last_count:
                    timestamp = time.strftime("%H:%M:%S")
                    print(f"[{timestamp}] Phase3 Units: {count}")
                    last_count = count
            else:
                print(f"[{time.strftime('%H:%M:%S')}] ❌ API unreachable")
            
            time.sleep(interval)
    except KeyboardInterrupt:
        print("\n\n✨ Monitoring stopped")

def main():
    parser = argparse.ArgumentParser(description="Monitor Mint ingot count")
    parser.add_argument("--watch", action="store_true", help="Continuous monitoring")
    parser.add_argument("--interval", type=int, default=10, help="Watch interval (seconds)")
    args = parser.parse_args()
    
    monitor(args.watch, args.interval)

if __name__ == "__main__":
    main()
