#!/usr/bin/env python3
"""
Integration Test: Vault emits distostream authorization on Phase3 batch

Expected flow (MVP2):
- Publish a valid vault.phase3.completed batch
- Vault stores certificates and emits vault.distostream.authorized
- Event includes: event_type, contract_id, robotorq_total, duration_seconds, provenance_cert_ids
"""

import asyncio
import json
import time
from typing import Optional

from nats.aio.client import Client as NATS

NATS_URL = "nats://localhost:4222"
VAULT_PHASE3_SUBJECT = "vault.phase3.completed"
DISTO_AUTH_SUBJECT = "vault.distostream.authorized"


def create_valid_batch(batch_id: str, cert_count: int = 2, contract_id: str = "test-contract") -> dict:
    now_ms = int(time.time() * 1000)
    certs = [
        {
            "cert_id": f"cert-{batch_id}-{i}",
            "merkle_root": f"merkle-{batch_id}-{i:04d}",
            "tree_height": 10,
            "contract_ids": [contract_id],
            "minted_at": now_ms,
        }
        for i in range(cert_count)
    ]
    return {
        "event_type": "robotorqcert_batch_completed",
        "batch_id": batch_id,
        "created_at": now_ms,
        "cert_count": cert_count,
        "total_robostake": 0,
        "canonical_total_jouletorq": cert_count * 3_600_000,
        "certificates": certs,
    }


async def wait_for_authorization(nc: NATS, timeout: float = 5.0) -> Optional[dict]:
    fut: asyncio.Future = asyncio.get_running_loop().create_future()

    async def handler(msg):
        try:
            data = json.loads(msg.data.decode())
            fut.set_result(data)
        except Exception as e:
            fut.set_exception(e)

    sub = await nc.subscribe(DISTO_AUTH_SUBJECT, cb=handler)
    try:
        return await asyncio.wait_for(fut, timeout=timeout)
    except asyncio.TimeoutError:
        return None
    finally:
        await sub.unsubscribe()


async def main():
    nc = NATS()
    await nc.connect(NATS_URL)

    # Publish a valid batch
    batch_id = "auth-test-0001"
    batch = create_valid_batch(batch_id=batch_id, cert_count=2)

    # Start listening for authorization before publishing
    waiter = asyncio.create_task(wait_for_authorization(nc))
    # Give the subscribe coroutine a moment to install the subscription
    await asyncio.sleep(0.1)

    await nc.publish(VAULT_PHASE3_SUBJECT, json.dumps(batch).encode())

    event = await waiter
    assert event is not None, "Expected vault.distostream.authorized event"

    # Minimal field checks for MVP2
    assert event.get("event_type") == "distostream_authorized"
    assert event.get("robotorq_total") == 2

    # Contract id can be derived from first certificate for MVP2
    assert event.get("contract_id") in ("test-contract", None)

    # Default duration expected to be ~60s unless configured otherwise
    duration = event.get("duration_seconds")
    assert isinstance(duration, int), "duration_seconds must be int"
    assert duration > 0, "duration_seconds must be positive"

    # Provenance optional but nice-to-have
    prov = event.get("provenance_cert_ids", [])
    assert isinstance(prov, list)

    await nc.close()
    print("✅ distostream authorization test passed")


if __name__ == "__main__":
    asyncio.run(main())
