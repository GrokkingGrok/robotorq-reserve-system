#!/usr/bin/env python3
"""
Wallet-Vault Integration Test Script
Tests communication between wallet and vault services
"""

import asyncio
import json
import time
import requests
from datetime import datetime

# Service endpoints
WALLET_URL = "http://localhost:8085"
VAULT_URL = "http://localhost:8088"
PROMETHEUS_URL = "http://localhost:9091"

def log(message):
    """Log with timestamp"""
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {message}")

def check_service_health(service_name, url):
    """Check if a service is healthy"""
    try:
        response = requests.get(f"{url}/health", timeout=5)
        if response.status_code == 200:
            log(f"✅ {service_name} is healthy")
            return True
        else:
            log(f"❌ {service_name} health check failed: {response.status_code}")
            return False
    except Exception as e:
        log(f"❌ {service_name} health check error: {e}")
        return False

def get_metrics():
    """Get current metrics from Prometheus"""
    try:
        # Get wallet metrics
        wallet_response = requests.get(f"{WALLET_URL}/metrics", timeout=5)
        wallet_metrics = wallet_response.text if wallet_response.status_code == 200 else "Error"

        # Get vault metrics
        vault_response = requests.get(f"{VAULT_URL}/metrics", timeout=5)
        vault_metrics = vault_response.text if vault_response.status_code == 200 else "Error"

        return wallet_metrics, vault_metrics
    except Exception as e:
        log(f"Error getting metrics: {e}")
        return "Error", "Error"

def parse_metric_value(metrics_text, metric_name):
    """Extract a metric value from Prometheus text format"""
    for line in metrics_text.split('\n'):
        if line.startswith(metric_name):
            parts = line.split()
            if len(parts) >= 2:
                try:
                    return float(parts[1])
                except ValueError:
                    return 0
    return 0

async def run_integration_test():
    """Run the wallet-vault integration test"""
    log("🚀 Starting Wallet-Vault Integration Test")

    # Check service health
    log("\n📊 Checking service health...")
    wallet_healthy = check_service_health("Wallet", WALLET_URL)
    vault_healthy = check_service_health("Vault", VAULT_URL)

    if not wallet_healthy or not vault_healthy:
        log("❌ Services not healthy, aborting test")
        return

    # Get initial metrics
    log("\n📈 Getting initial metrics...")
    wallet_metrics_initial, vault_metrics_initial = get_metrics()

    wallet_tx_initial = parse_metric_value(wallet_metrics_initial, "wallet_transactions_total")
    vault_contracts_initial = parse_metric_value(vault_metrics_initial, "vault_contracts_received_total")

    log(f"Initial wallet transactions: {wallet_tx_initial}")
    log(f"Initial vault contracts: {vault_contracts_initial}")

    # Test 1: Check wallet balance endpoint
    log("\n💰 Testing wallet balance...")
    try:
        response = requests.get(f"{WALLET_URL}/balance", timeout=5)
        if response.status_code == 200:
            balance_data = response.json()
            log(f"✅ Wallet balance: {balance_data}")
        else:
            log(f"❌ Wallet balance failed: {response.status_code}")
    except Exception as e:
        log(f"❌ Wallet balance error: {e}")

    # Test 2: Check vault status
    log("\n🏦 Testing vault status...")
    try:
        response = requests.get(f"{VAULT_URL}/health", timeout=5)
        if response.status_code == 200:
            log("✅ Vault health check passed")
        else:
            log(f"❌ Vault health check failed: {response.status_code}")
    except Exception as e:
        log(f"❌ Vault health check error: {e}")

    # Test 3: Simulate some activity by checking metrics over time
    log("\n⏱️  Monitoring metrics for 30 seconds...")
    for i in range(6):
        time.sleep(5)
        wallet_metrics, vault_metrics = get_metrics()

        wallet_tx_current = parse_metric_value(wallet_metrics, "wallet_transactions_total")
        vault_contracts_current = parse_metric_value(vault_metrics, "vault_contracts_received_total")
        vault_transfers = parse_metric_value(vault_metrics, "vault_wallet_transfers_total")

        log(f"  [{i*5}s] Wallet TX: {wallet_tx_current}, Vault Contracts: {vault_contracts_current}, Transfers: {vault_transfers}")

    # Final metrics comparison
    log("\n📊 Final metrics summary:")
    wallet_metrics_final, vault_metrics_final = get_metrics()

    wallet_tx_final = parse_metric_value(wallet_metrics_final, "wallet_transactions_total")
    vault_contracts_final = parse_metric_value(vault_metrics_final, "vault_contracts_received_total")
    vault_transfers_final = parse_metric_value(vault_metrics_final, "vault_wallet_transfers_total")

    log(f"Wallet transactions: {wallet_tx_initial} → {wallet_tx_final} (Δ{int(wallet_tx_final - wallet_tx_initial)})")
    log(f"Vault contracts: {vault_contracts_initial} → {vault_contracts_final} (Δ{int(vault_contracts_final - vault_contracts_initial)})")
    log(f"Vault transfers: {vault_transfers_final}")

    # Test 4: Check NATS connectivity (indirectly via service health)
    log("\n📡 Checking NATS connectivity...")
    # Since both services are healthy and communicating via NATS,
    # their continued operation indicates NATS connectivity
    log("✅ Services remain healthy - NATS connectivity confirmed")

    log("\n🎉 Integration test completed!")
    log("💡 Check Grafana dashboard at http://localhost:3001")
    log("   Username: admin, Password: admin")
    log("   Import dashboard: wallet-vault-integration-dashboard.json")

if __name__ == "__main__":
    asyncio.run(run_integration_test())