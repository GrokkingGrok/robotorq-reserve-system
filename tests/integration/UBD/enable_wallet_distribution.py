#!/usr/bin/env python3
"""
Enable Wallet Distribution - Turn on the RT flow to wallets

This script flips the switch to start sending RoboTorq units from DistoDam to Wallet.

What it does:
1. Updates docker-compose.yaml to set WALLET_DISTRIBUTION_ENABLED=true
2. Restarts DistoDam service to pick up new config
3. Verifies wallet distribution is active

Usage:
    python scripts/enable_wallet_distribution.py

Optional arguments:
    --rt-per-min AMOUNT    Set RT distribution rate (default: 0.001)
    --dry-run              Show what would change without applying
    --disable              Turn OFF distribution instead

Examples:
    # Increase to 0.01 RT per minute
    python scripts/enable_wallet_distribution.py --rt-per-min 0.01

    # Just check what would change
    python scripts/enable_wallet_distribution.py --dry-run
"""

import argparse
import subprocess
import sys
import time
import re
from pathlib import Path

# ANSI colors
GREEN = '\033[92m'
YELLOW = '\033[93m'
RED = '\033[91m'
BLUE = '\033[94m'
RESET = '\033[0m'

def print_success(msg):
    print(f"{GREEN}✅ {msg}{RESET}")

def print_warning(msg):
    print(f"{YELLOW}⚠️  {msg}{RESET}")

def print_error(msg):
    print(f"{RED}❌ {msg}{RESET}")

def print_info(msg):
    print(f"{BLUE}ℹ️  {msg}{RESET}")

def run_command(cmd, check=True, capture_output=True):
    """Run shell command and return result"""
    try:
        result = subprocess.run(
            cmd,
            shell=True,
            check=check,
            capture_output=capture_output,
            text=True
        )
        return result.stdout if capture_output else None
    except subprocess.CalledProcessError as e:
        if check:
            print_error(f"Command failed: {cmd}")
            print(e.stderr)
            sys.exit(1)
        return None

def find_docker_compose():
    """Find docker-compose.yaml file"""
    compose_path = Path("docker-compose.yaml")
    if not compose_path.exists():
        print_error("docker-compose.yaml not found in current directory")
        print_info("Please run this script from the robotorq-network root")
        sys.exit(1)
    return compose_path

def read_docker_compose(compose_path):
    """Read docker-compose.yaml content"""
    with open(compose_path, 'r') as f:
        return f.read()

def write_docker_compose(compose_path, content):
    """Write updated docker-compose.yaml"""
    with open(compose_path, 'w') as f:
        f.write(content)

def update_distribution_config(content, enabled=True, rt_per_min=None):
    """Update WALLET_DISTRIBUTION_ENABLED in docker-compose.yaml"""
    
    # Find the distodam service environment section
    distodam_section_pattern = r'(  distodam:.*?environment:.*?)(- WALLET_DISTRIBUTION_ENABLED=)(true|false)'
    
    match = re.search(distodam_section_pattern, content, re.DOTALL)
    if not match:
        print_error("Could not find WALLET_DISTRIBUTION_ENABLED in docker-compose.yaml")
        print_info("Expected format: '- WALLET_DISTRIBUTION_ENABLED=true' under distodam environment")
        sys.exit(1)
    
    # Replace enabled/disabled
    new_value = 'true' if enabled else 'false'
    updated_content = re.sub(
        r'(- WALLET_DISTRIBUTION_ENABLED=)(true|false)',
        f'\\1{new_value}',
        content
    )
    
    # Update RT per minute if specified
    if rt_per_min is not None:
        updated_content = re.sub(
            r'(- WALLET_DISTRIBUTION_RT_PER_MIN=)[\d.]+',
            f'\\1{rt_per_min}',
            updated_content
        )
    
    return updated_content

def get_current_config(content):
    """Extract current wallet distribution config"""
    enabled_match = re.search(r'- WALLET_DISTRIBUTION_ENABLED=(true|false)', content)
    rate_match = re.search(r'- WALLET_DISTRIBUTION_RT_PER_MIN=([\d.]+)', content)
    
    return {
        'enabled': enabled_match.group(1) == 'true' if enabled_match else False,
        'rt_per_min': float(rate_match.group(1)) if rate_match else 0.001
    }

def restart_distodam():
    """Restart DistoDam service to pick up new config"""
    print_info("Restarting DistoDam service...")
    
    # Stop
    run_command("docker-compose stop distodam", check=False)
    time.sleep(2)
    
    # Start
    run_command("docker-compose up -d distodam")
    time.sleep(5)
    
    # Check health
    result = run_command("docker-compose ps distodam", check=False)
    if result and "Up" in result:
        print_success("DistoDam restarted successfully")
        return True
    else:
        print_error("DistoDam failed to start")
        print_info("Check logs: docker logs robotorq-network-distodam-1")
        return False

def verify_distribution_active():
    """Check DistoDam logs to confirm distribution is active"""
    print_info("Verifying wallet distribution is active...")
    
    time.sleep(3)  # Wait for startup
    
    logs = run_command("docker logs robotorq-network-distodam-1 --tail 50", check=False)
    if not logs:
        print_warning("Could not retrieve DistoDam logs")
        return False
    
    # Look for distribution startup message
    if "wallet distribution enabled" in logs.lower() or "starting wallet distributor" in logs.lower():
        print_success("Wallet distribution is ACTIVE")
        
        # Extract RT per minute from logs
        rate_match = re.search(r'rt_per_minute["\s:]+([0-9.]+)', logs)
        if rate_match:
            rate = rate_match.group(1)
            print_info(f"Distribution rate: {rate} RT per minute")
        
        return True
    elif "wallet distribution disabled" in logs.lower():
        print_warning("Wallet distribution is still DISABLED")
        print_info("Check docker-compose.yaml was updated correctly")
        return False
    else:
        print_warning("Could not confirm distribution status from logs")
        print_info("Check logs manually: docker logs robotorq-network-distodam-1")
        return False

def check_wallet_service():
    """Verify wallet service is running"""
    print_info("Checking wallet service status...")
    
    result = run_command("docker-compose ps wallet", check=False)
    if result and "Up" in result:
        print_success("Wallet service is running")
        return True
    else:
        print_error("Wallet service is not running")
        print_info("Start it with: docker-compose up -d wallet")
        return False

def show_monitoring_commands():
    """Display commands to monitor the flow"""
    print("\n" + "="*60)
    print(f"{BLUE}📊 Monitor RT Distribution:{RESET}")
    print("="*60)
    print()
    print(f"{YELLOW}1. Watch DistoDam logs for distribution events:{RESET}")
    print("   docker logs -f robotorq-network-distodam-1 | grep -i distribution")
    print()
    print(f"{YELLOW}2. Watch Wallet logs for incoming RT:{RESET}")
    print("   docker logs -f robotorq-network-wallet-1 | grep -i 'rt_unit\\|distribution'")
    print()
    print(f"{YELLOW}3. Check wallet balance:{RESET}")
    print("   curl http://localhost:8085/wallet/test-wallet-001")
    print()
    print(f"{YELLOW}4. Prometheus metrics:{RESET}")
    print("   curl http://localhost:8082/metrics | grep wallet_distribution")
    print("   curl http://localhost:8085/metrics | grep rt_units_received")
    print()
    print("="*60)

def main():
    parser = argparse.ArgumentParser(
        description="Enable/disable wallet distribution in RoboTorq network",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        '--rt-per-min',
        type=float,
        help='Set RT distribution rate per minute (default: 0.001)'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would change without applying'
    )
    parser.add_argument(
        '--disable',
        action='store_true',
        help='Disable distribution instead of enabling'
    )
    parser.add_argument(
        '--skip-restart',
        action='store_true',
        help='Update config but skip service restart'
    )
    
    args = parser.parse_args()
    
    # Header
    action = "DISABLE" if args.disable else "ENABLE"
    print()
    print("="*60)
    print(f"{BLUE}🔧 {action} Wallet Distribution{RESET}")
    print("="*60)
    print()
    
    # Find docker-compose.yaml
    compose_path = find_docker_compose()
    print_info(f"Found docker-compose.yaml: {compose_path.absolute()}")
    
    # Read current config
    content = read_docker_compose(compose_path)
    current = get_current_config(content)
    
    print_info(f"Current status: {'ENABLED' if current['enabled'] else 'DISABLED'}")
    print_info(f"Current rate: {current['rt_per_min']} RT/min")
    print()
    
    # Check if change needed
    target_enabled = not args.disable
    if current['enabled'] == target_enabled and args.rt_per_min is None:
        print_warning(f"Distribution already {action}D")
        if not args.disable:
            show_monitoring_commands()
        sys.exit(0)
    
    # Update config
    rt_per_min = args.rt_per_min if args.rt_per_min is not None else current['rt_per_min']
    updated_content = update_distribution_config(content, enabled=target_enabled, rt_per_min=rt_per_min)
    
    # Show changes
    print(f"{YELLOW}Changes:{RESET}")
    print(f"  WALLET_DISTRIBUTION_ENABLED: {current['enabled']} → {target_enabled}")
    if args.rt_per_min is not None:
        print(f"  WALLET_DISTRIBUTION_RT_PER_MIN: {current['rt_per_min']} → {rt_per_min}")
    print()
    
    # Dry run exit
    if args.dry_run:
        print_info("DRY RUN - No changes applied")
        sys.exit(0)
    
    # Apply changes
    print_info("Updating docker-compose.yaml...")
    write_docker_compose(compose_path, updated_content)
    print_success("Configuration updated")
    
    # Skip restart if requested
    if args.skip_restart:
        print_warning("Skipping service restart (--skip-restart)")
        print_info("Restart manually: docker-compose restart distodam")
        sys.exit(0)
    
    # Restart DistoDam
    if not restart_distodam():
        sys.exit(1)
    
    # Verify
    if target_enabled:
        verify_distribution_active()
        check_wallet_service()
        show_monitoring_commands()
    else:
        print_success("Wallet distribution DISABLED")
        print_info("No RT units will be sent to wallets")
    
    print()
    print_success("✨ Done!")
    print()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
        print_warning("Interrupted by user")
        sys.exit(1)
    except Exception as e:
        print_error(f"Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
