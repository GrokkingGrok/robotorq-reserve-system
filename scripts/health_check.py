#!/usr/bin/env python3
"""
Check health of all RoboTorq services
Usage: python health_check.py
"""

import requests

SERVICES = {
    "Digger": "http://localhost:3030/health",
    "Refinery": "http://localhost:8081/health",
    "DistoDam": "http://localhost:8083/health",
}

def check_all_services():
    """Check health of all services"""
    print(f"\n{'='*60}")
    print(f"RoboTorq Services - Health Check")
    print(f"{'='*60}\n")
    
    results = {}
    
    for service, url in SERVICES.items():
        try:
            response = requests.get(url, timeout=2)
            response.raise_for_status()
            status = "✅ HEALTHY"
            results[service] = True
        except requests.exceptions.RequestException:
            status = "❌ UNREACHABLE"
            results[service] = False
        
        print(f"{service:15} {status}")
    
    print(f"\n{'='*60}")
    healthy = sum(results.values())
    total = len(results)
    print(f"Status: {healthy}/{total} services healthy")
    print(f"{'='*60}\n")
    
    return all(results.values())

if __name__ == "__main__":
    check_all_services()
