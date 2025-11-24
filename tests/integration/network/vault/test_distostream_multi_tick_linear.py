#!/usr/bin/env python3
"""
Integration Test: Linear multi-tick distribution schedule
Requires distostream vault supporting distribution_mode="linear".

Flow:
- Publish authorization event with distribution_mode="linear", small duration (3s) and total units (5)
- Expect multiple ticks until cumulative_distributed == total
Assertions per tick:
- event_type == distostream_distribution_tick
- cumulative_distributed increases
- remaining decreases to 0
- final cumulative == total
"""
import asyncio
import json
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
AUTH_SUBJECT = "vault.distostream.authorized"
TICK_SUBJECT = "vault.distostream.distribution"

async def collect_ticks(nc: NATS, expected_total: int, timeout: float = 8.0):
    ticks = []
    fut = asyncio.get_running_loop().create_future()

    async def handler(msg):
        data = json.loads(msg.data.decode())
        if data.get("event_type") == "distostream_distribution_tick" and data.get("authorized_robotorq_total") == expected_total:
            ticks.append(data)
            if data.get("cumulative_distributed") == expected_total:
                if not fut.done():
                    fut.set_result(True)

    sub = await nc.subscribe(TICK_SUBJECT, cb=handler)
    try:
        await asyncio.wait_for(fut, timeout=timeout)
    except asyncio.TimeoutError:
        pass
    await sub.unsubscribe()
    return ticks

async def main():
    nc = NATS(); await nc.connect(NATS_URL)

    total = 5
    # Start collection before publish
    collector = asyncio.create_task(collect_ticks(nc, total))
    await asyncio.sleep(0.1)

    auth_event = {
        "event_type": "distostream_authorized",
        "contract_id": "linear-test-contract-001",
        "robotorq_total": total,
        "tokentorq_remainder": 0,
        "jouletorq_remainder": 0,
        "duration_seconds": 3,  # 3 seconds window
        "distribution_mode": "linear",
        "provenance_cert_ids": ["synthetic-cert-1"],
    }
    await nc.publish(AUTH_SUBJECT, json.dumps(auth_event).encode())

    ticks = await collector
    await nc.close()

    assert len(ticks) >= 2, f"Expected multi ticks, got {len(ticks)}"
    cumulative = 0
    prev_remaining = None
    for i, t in enumerate(ticks):
        assert t.get("tick_index") == i, "Sequential tick_index mismatch"
        amt = t.get("distributed_robotorq")
        cumulative += amt
        assert t.get("cumulative_distributed") == cumulative, "Cumulative mismatch"
        remaining = t.get("remaining")
        if prev_remaining is not None:
            assert remaining <= prev_remaining, "Remaining should not increase"
        prev_remaining = remaining
    assert cumulative == total, "Final cumulative must equal total"
    assert ticks[-1].get("remaining") == 0, "Final remaining must be zero"
    print("✅ linear multi-tick distribution test passed")

if __name__ == "__main__":
    asyncio.run(main())
