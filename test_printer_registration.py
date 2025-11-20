#!/usr/bin/env python3
"""
Quick test: Send printer registration to Digger via NATS
"""

import asyncio
import json
from nats.aio.client import Client as NATS

async def test_registration():
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    
    # Mock printer registration
    registration = {
        "printer_id": "test-ender3-001",
        "model": "Ender3V3KE",
        "manufacturer": "Creality",
        "serial_number": "TEST123456",
        "rated_watts": 350,
        "public_key": "deadbeef1234567890abcdef"  # Mock Falcon-1024 pubkey
    }
    
    print("📤 Sending printer registration...")
    print(f"   Printer ID: {registration['printer_id']}")
    print(f"   Model: {registration['model']}")
    
    # Subscribe to response topic first
    response_topic = f"printer.{registration['printer_id']}.certificate"
    cert_received = asyncio.Event()
    received_cert = {}
    
    async def cert_handler(msg):
        nonlocal received_cert
        received_cert = json.loads(msg.data.decode())
        cert_received.set()
    
    await nc.subscribe(response_topic, cb=cert_handler)
    await asyncio.sleep(0.5)  # Give subscription time to be ready
    
    # Publish registration
    await nc.publish(
        "printer.register",
        json.dumps(registration).encode()
    )
    
    # Wait for response
    print("\n⏳ Waiting for certificate...")
    try:
        await asyncio.wait_for(cert_received.wait(), timeout=5)
    except asyncio.TimeoutError:
        print("❌ Timeout waiting for certificate")
        await nc.close()
        return
    
    print("\n✅ Certificate received!")
    print(f"   Printer ID: {received_cert['printer_id']}")
    print(f"   Model: {received_cert['model']}")
    print(f"   Rated Watts: {received_cert['rated_watts']}")
    print(f"   Issued At: {received_cert['issued_at']}")
    print(f"   Certificate Hash: {received_cert['certificate_hash'][:32]}...")
    print(f"   Signature: {received_cert['signature'][:32]}...")
    
    await nc.close()

if __name__ == "__main__":
    asyncio.run(test_registration())
