#!/usr/bin/env python3
"""
Activate Wallet for UBD

Sends a wallet activation message to DistoDam so it starts distributing RT.

Usage:
    python scripts/activate_wallet.py [wallet_id]
    
Default wallet_id: test-wallet-001
"""

import asyncio
import json
import sys
from datetime import datetime
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"

async def activate_wallet(wallet_id):
    """Send wallet activation to DistoDam"""
    nc = NATS()
    
    try:
        await nc.connect(NATS_URL)
        print(f"✅ Connected to NATS")
        
        # Publish activation message
        activation_message = {
            "wallet_id": wallet_id,
            "activate": True,
            "requested_at": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
        }
        
        await nc.publish(
            "wallet.activate",
            json.dumps(activation_message).encode()
        )
        
        print(f"✅ Wallet activated: {wallet_id}")
        print(f"📡 DistoDam will now send RT distributions to this wallet")
        print(f"\nMonitor distributions:")
        print(f"  docker logs -f robotorq-network-distodam-1 | grep '{wallet_id}'")
        print(f"  docker logs -f robotorq-network-wallet-1")
        
        await nc.close()
        return True
        
    except Exception as e:
        print(f"❌ Failed to activate wallet: {e}")
        if nc.is_connected:
            await nc.close()
        return False

if __name__ == "__main__":
    wallet_id = sys.argv[1] if len(sys.argv) > 1 else "test-wallet-001"
    
    print(f"\n🔔 Activating wallet: {wallet_id}\n")
    
    success = asyncio.run(activate_wallet(wallet_id))
    exit(0 if success else 1)
