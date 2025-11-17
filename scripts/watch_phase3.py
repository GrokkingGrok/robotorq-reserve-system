#!/usr/bin/env python3
"""
Watch for Phase3 RoboTorq units on NATS
Usage: python watch_phase3.py [--timeout SECONDS]
"""

import asyncio
import json
import argparse
import signal
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
PHASE3_TOPIC = "distodam.units"

shutdown = False

def signal_handler(sig, frame):
    global shutdown
    shutdown = True
    print("\n\n✨ Shutting down...")

async def watch_phase3_units(timeout=None):
    """Watch for Phase3 units on NATS"""
    global shutdown
    
    print(f"\n{'='*60}")
    print(f"Watching for Phase3 RoboTorq Units")
    print(f"Topic: {PHASE3_TOPIC}")
    if timeout:
        print(f"Timeout: {timeout}s")
    print(f"Press Ctrl+C to stop")
    print(f"{'='*60}\n")
    
    nc = NATS()
    
    try:
        await nc.connect(NATS_URL)
        print(f"✅ Connected to NATS\n")
        
        received_count = 0
        
        async def message_handler(msg):
            nonlocal received_count
            try:
                unit = json.loads(msg.data.decode())
                received_count += 1
                
                print(f"\n{'='*60}")
                print(f"📦 Phase3 RoboTorq Unit #{received_count}")
                print(f"{'='*60}")
                print(f"Unit ID: {unit.get('unit_id', 'N/A')}")
                print(f"JouleTorq Total: {unit.get('jouletorq_total', 0):.2f}")
                print(f"RoboTorq Total: {unit.get('robotorq_total', 0):.6f}")
                print(f"Ingots: {len(unit.get('phase2_ingots', []))}")
                print(f"Root Hash: {unit.get('merkle_root_hash', 'N/A')[:64]}...")
                
                has_signature = bool(unit.get('signature'))
                has_pubkey = bool(unit.get('public_key'))
                print(f"Signature: {'✅ Present' if has_signature else '❌ Missing'}")
                print(f"Public Key: {'✅ Present' if has_pubkey else '❌ Missing'}")
                print(f"{'='*60}\n")
                
            except Exception as e:
                print(f"❌ Error processing message: {e}")
        
        await nc.subscribe(PHASE3_TOPIC, cb=message_handler)
        
        # Wait for messages
        if timeout:
            await asyncio.sleep(timeout)
        else:
            while not shutdown:
                await asyncio.sleep(1)
        
        await nc.close()
        
        print(f"\n✨ Received {received_count} Phase3 unit(s)")
        
    except Exception as e:
        print(f"❌ Error: {e}")

def main():
    parser = argparse.ArgumentParser(description="Watch for Phase3 RoboTorq units")
    parser.add_argument("--timeout", type=int, help="Stop after N seconds")
    args = parser.parse_args()
    
    # Handle Ctrl+C gracefully
    signal.signal(signal.SIGINT, signal_handler)
    
    asyncio.run(watch_phase3_units(args.timeout))

if __name__ == "__main__":
    main()
