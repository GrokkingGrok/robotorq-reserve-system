#!/usr/bin/env python3
"""
Integration Test: DistoVault emits distribution tick upon authorization event

Flow (MVP2):
- Publish synthetic vault.distostream.authorized event
- ShadowDistoVault subscriber receives it and immediately publishes
  vault.distostream.distribution tick (tick_index=0, full amount distributed)

Assertions:
- One distribution tick received within timeout
- event_type == distostream_distribution_tick
- authorized_robotorq_total matches original robotorq_total
- distributed_robotorq == authorized_robotorq_total (MVP immediate distribution)
- tick_index == 0
"""
import asyncio
import json
from typing import Optional
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
DISTO_AUTH_SUBJECT = "vault.distostream.authorized"
DISTO_TICK_SUBJECT = "vault.distostream.distribution"

async def wait_for_tick(nc: NATS, timeout: float = 5.0) -> Optional[dict]:
    fut: asyncio.Future = asyncio.get_running_loop().create_future()

    async def handler(msg):
        try:
            data = json.loads(msg.data.decode())
            fut.set_result(data)
        except Exception as e:
            fut.set_exception(e)

    sub = await nc.subscribe(DISTO_TICK_SUBJECT, cb=handler)
    try:
        return await asyncio.wait_for(fut, timeout=timeout)
    except asyncio.TimeoutError:
        return None
    finally:
        await sub.unsubscribe()

async def main():
    nc = NATS()
    await nc.connect(NATS_URL)

    # Prepare to listen for distribution tick first
    waiter = asyncio.create_task(wait_for_tick(nc))
    await asyncio.sleep(0.1)  # ensure subscription active

    # Publish synthetic authorization event
    auth_event = {
        "event_type": "distostream_authorized",
        "contract_id": "distotest-contract-001",
        "robotorq_total": 5,
        "tokentorq_remainder": 0,
        "jouletorq_remainder": 0,
        "duration_seconds": 60,
        "provenance_cert_ids": ["synthetic-cert-1", "synthetic-cert-2"],
    }
    await nc.publish(DISTO_AUTH_SUBJECT, json.dumps(auth_event).encode())

    tick = await waiter
    assert tick is not None, "Expected distribution tick event"
    assert tick.get("event_type") == "distostream_distribution_tick"
    assert tick.get("contract_id") == auth_event["contract_id"]
    assert tick.get("tick_index") == 0
    assert tick.get("authorized_robotorq_total") == auth_event["robotorq_total"]
    assert tick.get("distributed_robotorq") == auth_event["robotorq_total"]

    await nc.close()
    print("✅ distostream distribution tick test passed")

if __name__ == "__main__":
    asyncio.run(main())
