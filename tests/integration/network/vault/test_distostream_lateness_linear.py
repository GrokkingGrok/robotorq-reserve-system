#!/usr/bin/env python3
"""
Integration Test: Linear multi-tick distribution includes lateness tracking.

Verifies each tick event contains planned_at_ms, executed_at_ms, lateness_ms >= 0.
Ensures final cumulative equals total and lateness is bounded (< 500ms typical).
"""
import asyncio
import json
from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
AUTH_SUBJECT = "vault.distostream.authorized"
TICK_SUBJECT = "vault.distostream.distribution"

async def collect_ticks(nc: NATS, total: int, timeout: float = 8.0):
    ticks = []
    fut = asyncio.get_running_loop().create_future()

    async def handler(msg):
        data = json.loads(msg.data.decode())
        if data.get("event_type") != "distostream_distribution_tick":
            return
        if data.get("authorized_robotorq_total") != total:
            return
        ticks.append(data)
        if data.get("cumulative_distributed") == total:
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
    total = 7
    collector_task = asyncio.create_task(collect_ticks(nc, total))
    await asyncio.sleep(0.1)
    auth_event = {
        "event_type": "distostream_authorized",
        "contract_id": "lateness-test-contract-001",
        "robotorq_total": total,
        "tokentorq_remainder": 0,
        "jouletorq_remainder": 0,
        "duration_seconds": 4,
        "distribution_mode": "linear",
        "provenance_cert_ids": ["synthetic-cert-1"],
    }
    await nc.publish(AUTH_SUBJECT, json.dumps(auth_event).encode())
    ticks = await collector_task
    await nc.close()

    assert len(ticks) >= 2, f"Expected multi ticks, got {len(ticks)}"
    cumulative = 0
    for i, t in enumerate(ticks):
        assert t.get("tick_index") == i, "Sequential tick index mismatch"
        assert "planned_at_ms" in t, "Missing planned_at_ms"
        assert "executed_at_ms" in t, "Missing executed_at_ms"
        assert "lateness_ms" in t, "Missing lateness_ms"
        assert t["lateness_ms"] >= 0, "lateness_ms should be >= 0"
        # Heuristic bound (not strict): should not exceed 2000ms under normal load
        assert t["lateness_ms"] < 2000, f"lateness too high: {t['lateness_ms']}"
        cumulative += t.get("distributed_robotorq")
        assert cumulative == t.get("cumulative_distributed"), "Cumulative mismatch"
    assert cumulative == total, "Final cumulative must equal total"
    assert ticks[-1].get("remaining") == 0, "Final remaining must be zero"
    print("✅ lateness linear multi-tick test passed")

if __name__ == "__main__":
    asyncio.run(main())
