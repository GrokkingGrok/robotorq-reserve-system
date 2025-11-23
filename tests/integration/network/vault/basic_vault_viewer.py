#!/usr/bin/env python3
"""
Basic Vault Viewer Integration Test
Watches x batches arrive, separated by y seconds, configurable via command line.

Prerequisites:
- docker-compose up -d (vault, nats running)
- python -m pip install nats-py requests (optional for metrics polling)

Usage:
  python tests/integration/network/vault/basic_vault_viewer.py --batches 5 --delay 2

This test:
1. Publishes x RoboTorqBatches to vault.phase3.completed subject
2. Waits y seconds between each batch
3. Polls vault /health and /metrics after each batch
4. Prints vault state for observation
"""

import argparse
import asyncio
import json
import time
import sys
from typing import Tuple

import nats
from nats.aio.client import Client as NATS

# Vault config
NATS_URL = "nats://localhost:4222"
VAULT_PHASE3_SUBJECT = "vault.phase3.completed"
VAULT_HEALTH_URL = "http://localhost:8088/health"
VAULT_METRICS_URL = "http://localhost:8088/metrics"

def _http_get(url: str) -> Tuple[int, str]:
    try:
        import requests
        r = requests.get(url, timeout=3)
        return r.status_code, r.text
    except Exception:
        # Fallback to urllib
        import urllib.request
        try:
            with urllib.request.urlopen(url, timeout=3) as resp:
                return resp.getcode(), resp.read().decode()
        except Exception as e:
            return 0, str(e)

def create_sample_batch(batch_id: str, cert_count: int = 1, total_robostake: int = 3) -> dict:
    """Generate a sample RoboTorqBatch for testing."""
    certificates = [
        {
            "cert_id": f"cert-{batch_id}-{i}",
            "contract_ids": ["test-contract"],
            "jouletorq_units": 1000,
            "timestamp": "2025-11-23T00:00:00Z"
        } for i in range(cert_count)
    ]
    return {
        "event_type": "phase3.completed",
        "batch_id": batch_id,
        "created_at": "2025-11-23T00:00:00Z",
        "cert_count": cert_count,
        "total_robostake": total_robostake,
        "canonical_total_jouletorq": cert_count * 1000,
        "certificates": certificates
    }

async def publish_batch(nc: NATS, batch: dict) -> None:
    """Publish a batch to NATS."""
    payload = json.dumps(batch).encode()
    await nc.publish(VAULT_PHASE3_SUBJECT, payload)
    print(f"Published batch {batch['batch_id']} with {batch['cert_count']} certs, {batch['total_robostake']} robostake")

async def poll_vault() -> None:
    """Poll vault health and metrics."""
    code, body = _http_get(VAULT_HEALTH_URL)
    if code == 200:
        health = json.loads(body)
        print(f"Vault Health: {health}")
    else:
        print(f"Health check failed: {code} {body}")

    code, body = _http_get(VAULT_METRICS_URL)
    if code == 200:
        # Extract key metrics
        lines = body.split('\n')
        metrics = {}
        for line in lines:
            if line.startswith('vault_'):
                parts = line.split(' ')
                if len(parts) >= 2:
                    metrics[parts[0]] = parts[1]
        print(f"Vault Metrics: {metrics}")
    else:
        print(f"Metrics fetch failed: {code} {body}")

async def main():
    parser = argparse.ArgumentParser(description="Basic Vault Viewer Test")
    parser.add_argument("--batches", type=int, default=3, help="Number of batches to send (x)")
    parser.add_argument("--delay", type=int, default=5, help="Seconds between batches (y)")
    args = parser.parse_args()

    print(f"Starting vault viewer test: {args.batches} batches, {args.delay}s delay")

    # Connect to NATS
    nc = NATS()
    await nc.connect(NATS_URL)
    print("Connected to NATS")

    # Initial poll
    await poll_vault()

    # Send batches
    for i in range(1, args.batches + 1):
        batch_id = f"viewer-batch-{i:03d}"
        batch = create_sample_batch(batch_id, cert_count=1, total_robostake=3)
        await publish_batch(nc, batch)
        await asyncio.sleep(args.delay)
        await poll_vault()

    # Final poll
    await asyncio.sleep(1)
    await poll_vault()

    await nc.close()
    print("Test completed")

if __name__ == "__main__":
    asyncio.run(main())
