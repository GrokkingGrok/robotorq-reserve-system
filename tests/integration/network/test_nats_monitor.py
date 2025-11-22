#!/usr/bin/env python3
"""
Simple NATS monitor to verify messages are reaching the server
"""

import asyncio
import json
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"

async def main():
    print("🔌 Connecting to NATS...")
    nc = NATS()
    await nc.connect(NATS_URL)
    print(f"✅ Connected to {NATS_URL}")
    
    message_count = 0
    
    async def message_handler(msg):
        nonlocal message_count
        message_count += 1
        print(f"📨 Message #{message_count} on subject: {msg.subject}")
        print(f"   Size: {len(msg.data)} bytes")
        try:
            data = json.loads(msg.data.decode())
            print(f"   Data: {json.dumps(data, indent=2)}")
        except:
            print(f"   Raw: {msg.data[:100]}")
    
    # Subscribe to all printer topics
    print("\n👂 Subscribing to printer.>")
    await nc.subscribe("printer.>", cb=message_handler)
    
    print("\n✅ Monitoring all printer.* topics")
    print("   Press Ctrl+C to stop\n")
    
    try:
        await asyncio.Future()  # Run forever
    except KeyboardInterrupt:
        print(f"\n\n📊 Received {message_count} messages total")
    finally:
        await nc.close()

if __name__ == "__main__":
    asyncio.run(main())
