#!/usr/bin/env python3
"""Integration Test: Vault /health and /metrics endpoints.

Ensures the Vault service started via docker-compose exposes:
  - /health JSON with keys: certificates, available_robostake, deployed_robostake
  - /metrics plaintext including vault_* metric names

Prerequisites:
  docker-compose up -d vault
  python -m pip install requests (optional; falls back to urllib if missing)
"""

import json
import time
import sys
from typing import Tuple

VAULT_HOST = "http://localhost:8088"
HEALTH_URL = f"{VAULT_HOST}/health"
METRICS_URL = f"{VAULT_HOST}/metrics"
TIMEOUT_SEC = 15

def _http_get(url: str) -> Tuple[int, str]:
    try:
        import requests  # type: ignore
        r = requests.get(url, timeout=3)
        return r.status_code, r.text
    except Exception:
        # Fallback to urllib
        import urllib.request
        try:
            with urllib.request.urlopen(url, timeout=3) as resp:  # noqa: S310
                return resp.getcode(), resp.read().decode()
        except Exception as e:  # noqa: BLE001
            return 0, str(e)

def wait_for_health(timeout: int = TIMEOUT_SEC) -> bool:
    start = time.time()
    while time.time() - start < timeout:
        code, body = _http_get(HEALTH_URL)
        if code == 200:
            try:
                data = json.loads(body)
                if all(k in data for k in ["certificates", "available_robostake", "deployed_robostake"]):
                    return True
            except json.JSONDecodeError:
                pass
        time.sleep(1)
    return False

def test_health():
    assert wait_for_health(), f"Vault /health not ready within {TIMEOUT_SEC}s"
    code, body = _http_get(HEALTH_URL)
    assert code == 200, "Expected 200 from /health"
    data = json.loads(body)
    assert isinstance(data["certificates"], int)
    assert isinstance(data["available_robostake"], int)
    assert isinstance(data["deployed_robostake"], int)

def test_metrics():
    code, body = _http_get(METRICS_URL)
    assert code == 200, "Expected 200 from /metrics"
    # Basic presence checks
    assert "vault_cert_stored_total" in body
    assert "vault_robostake_returned_total" in body
    assert "vault_stake_allocated_total" in body
    assert "vault_available_robostake" in body
    assert "vault_deployed_robostake" in body

if __name__ == "__main__":  # Manual run
    ok = wait_for_health()
    if not ok:
        print("[FAIL] Vault /health not ready")
        sys.exit(1)
    test_health()
    test_metrics()
    print("[PASS] Vault health + metrics integration tests successful")