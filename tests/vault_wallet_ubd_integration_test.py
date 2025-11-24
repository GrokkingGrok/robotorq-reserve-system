#!/usr/bin/env python3
"""
Vault-to-Wallet UBD Integration Test
Tests the complete UBD distribution flow from vault to wallet with triple balance arithmetic verification.
"""

import asyncio
import json
import time
import sys
import os
import subprocess
import requests
from typing import Dict, Any, Optional

# Test configuration
TEST_WALLET_ID = "test-wallet-001"
TEST_AMOUNT = 1000  # jouletorq to distribute

# Service URLs
WALLET_URL = "http://localhost:8085"
VAULT_URL = "http://localhost:8088"
NATS_URL = "http://localhost:8222"

# Colors for output
class Colors:
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    END = '\033[0m'

def print_success(message: str):
    print(f"{Colors.GREEN}✅ {message}{Colors.END}")

def print_error(message: str):
    print(f"{Colors.RED}❌ {message}{Colors.END}")

def print_warning(message: str):
    print(f"{Colors.YELLOW}⚠️  {message}{Colors.END}")

def print_step(message: str):
    print(f"{Colors.BLUE}▶  {message}{Colors.END}")

def print_section(message: str):
    print(f"{Colors.CYAN}{'='*50}")
    print(f"{Colors.CYAN}{message}{Colors.CYAN}")
    print(f"{Colors.CYAN}{'='*50}{Colors.END}")

def wait_for_service(url: str, timeout: int = 30) -> bool:
    """Wait for a service to become available."""
    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            response = requests.get(url, timeout=5)
            if response.status_code == 200:
                return True
        except requests.RequestException:
            pass
        time.sleep(1)
    return False

def test_docker_container(container_name: str) -> bool:
    """Check if a Docker container is running."""
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", f"name={container_name}", "--format", "{{.Names}}"],
            capture_output=True, text=True, check=True
        )
        return container_name in result.stdout
    except subprocess.CalledProcessError:
        return False

def http_get(url: str) -> Optional[Dict[str, Any]]:
    """Make an HTTP GET request."""
    try:
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        return response.json()
    except requests.RequestException as e:
        print_error(f"HTTP GET failed: {url} - {e}")
        return None

def http_post(url: str, data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Make an HTTP POST request."""
    try:
        response = requests.post(url, json=data, timeout=10)
        response.raise_for_status()
        return response.json()
    except requests.RequestException as e:
        print_error(f"HTTP POST failed: {url} - {e}")
        return None

def http_get_text(url: str) -> Optional[str]:
    """Make an HTTP GET request and return raw text."""
    try:
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        return response.text.strip()
    except requests.RequestException as e:
        print_error(f"HTTP GET failed: {url} - {e}")
        return None

def get_wallet_balance(wallet_url: str) -> Optional[Dict[str, Any]]:
    """Get wallet balance."""
    balance = http_get(f"{wallet_url}/balance")
    if balance and 'balance' in balance:
        return {
            'robotorq': int(balance['balance']['robotorq']),
            'tokentorq_remainder': int(balance['balance']['tokentorq_remainder']),
            'jouletorq_remainder': int(balance['balance']['jouletorq_remainder']),
            'canonical_jouletorq': int(balance['balance']['canonical_jouletorq'])
        }
    return None

def test_triple_arithmetic(initial: Dict[str, int], final: Dict[str, int], expected_jouletorq: int) -> bool:
    """Test triple balance arithmetic."""
    initial_total = (initial['robotorq'] * 3600000 +
                     initial['tokentorq_remainder'] * 3600 +
                     initial['jouletorq_remainder'])
    final_total = (final['robotorq'] * 3600000 +
                   final['tokentorq_remainder'] * 3600 +
                   final['jouletorq_remainder'])
    received = final_total - initial_total

    print_step("Triple arithmetic verification:")
    print(f"  {Colors.YELLOW}Initial: {initial['robotorq']}R {initial['tokentorq_remainder']}T {initial['jouletorq_remainder']}J ({initial_total} J){Colors.END}")
    print(f"  {Colors.YELLOW}Final: {final['robotorq']}R {final['tokentorq_remainder']}T {final['jouletorq_remainder']}J ({final_total} J){Colors.END}")
    print(f"  {Colors.YELLOW}Received: {received} J (expected: {expected_jouletorq} J){Colors.END}")

    if received == expected_jouletorq:
        print_success("Triple arithmetic is correct!")
        return True
    else:
        print_error(f"Triple arithmetic mismatch! Expected {expected_jouletorq}, got {received}")
        return False

async def main():
    print_section("🚀 Starting Vault-to-Wallet UBD Integration Test")

    skip_build = "--skip-build" in sys.argv

    try:
        # Step 1: Build services if needed
        if not skip_build:
            print_step("Building services...")
            print_step("Building vault...")
            subprocess.run(["docker", "compose", "build", "vault"], check=True)

            print_step("Building wallet...")
            subprocess.run(["docker", "compose", "build", "wallet"], check=True)

            print_success("Services built successfully")

        # Step 2: Start services
        print_step("Starting services (NATS, PostgreSQL, Vault, Wallet)...")
        subprocess.run(["docker", "compose", "up", "-d", "nats", "postgres", "vault", "wallet"], check=True)

        # Step 3: Wait for services to be ready
        print_step("Waiting for services to be ready...")

        if not wait_for_service(f"{NATS_URL}", 30):
            raise Exception("NATS not ready")
        if not wait_for_service(f"{WALLET_URL}/health", 30):
            raise Exception("Wallet not ready")
        if not wait_for_service(f"{VAULT_URL}/health", 30):
            raise Exception("Vault not ready")

        # Verify containers are running
        services = ["robotorq-reserve-system-nats-1", "robotorq-reserve-system-postgres-1",
                   "robotorq-reserve-system-vault-1", "robotorq-reserve-system-wallet-1"]
        for service in services:
            if not test_docker_container(service):
                raise Exception(f"Service {service} is not running")
        print_success("All services are running")

        # Step 4: Get initial wallet balance
        print_step("Getting initial wallet balance...")
        initial_balance = get_wallet_balance(WALLET_URL)
        if not initial_balance:
            raise Exception("Failed to get initial wallet balance")
        print_success(f"Initial balance: {initial_balance['robotorq']}R {initial_balance['tokentorq_remainder']}T {initial_balance['jouletorq_remainder']}J")

        # Step 5: Activate wallet
        print_step("Activating wallet...")
        activate_result = http_get(f"{WALLET_URL}/activate")
        if not activate_result or not activate_result.get('activated'):
            raise Exception("Failed to activate wallet")
        print_success(f"Wallet activated: {activate_result['wallet_id']}")

        # Step 6: Trigger UBD distribution from vault
        print_step("Triggering UBD distribution from vault...")
        ubd_request = {
            "wallet_id": TEST_WALLET_ID,
            "amount_jouletorq": TEST_AMOUNT
        }
        ubd_result = http_post(f"{VAULT_URL}/ubd/distribute", ubd_request)
        if not ubd_result or not ubd_result.get('success'):
            raise Exception("Failed to trigger UBD distribution")
        print_success("UBD distribution triggered")

        # Step 7: Wait for UBD processing and check final balance
        print_step("Waiting for UBD processing...")
        time.sleep(5)  # Give time for async processing

        final_balance = get_wallet_balance(WALLET_URL)
        if not final_balance:
            raise Exception("Failed to get final wallet balance")

        # Step 8: Verify balance increase
        print_step("Verifying balance increase...")
        arithmetic_correct = test_triple_arithmetic(initial_balance, final_balance, TEST_AMOUNT)

        if arithmetic_correct:
            print_success("🎉 INTEGRATION TEST PASSED!")
            print(f"{Colors.GREEN}Vault-to-wallet UBD distribution working correctly{Colors.END}")
            print(f"{Colors.GREEN}Triple balance arithmetic verified{Colors.END}")
        else:
            raise Exception("Triple arithmetic verification failed")

        # Step 9: Test wallet health
        print_step("Testing wallet health...")
        health_result = http_get_text(f"{WALLET_URL}/health")
        if not health_result or health_result != "OK":
            raise Exception("Wallet health check failed")
        print_success("Wallet health: OK")

        # Step 10: Test vault health
        print_step("Testing vault health...")
        vault_health_result = http_get(f"{VAULT_URL}/health")
        if not vault_health_result:
            raise Exception("Vault health check failed")
        print_success("Vault health: OK")

    except Exception as e:
        print_error(f"Integration test failed: {e}")
        sys.exit(1)
    finally:
        # Cleanup
        keep_running = "--keep-running" in sys.argv
        if not keep_running:
            print_step("Cleaning up services...")
            subprocess.run(["docker", "compose", "down"])
            print_success("Services stopped")
        else:
            print_warning("Services left running (--keep-running specified)")

if __name__ == "__main__":
    asyncio.run(main())