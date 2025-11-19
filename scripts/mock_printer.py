#!/usr/bin/env python3
"""
Mock Printer - Simulates Physical RT Printer for Digger Testing

This script mimics a 3D printer that "prints" physical RT tokens by:
1. Receiving a print job (simulated by you running this script)
2. Telling Digger "printing started"
3. Periodically sending "milestone check" updates with fake photos
4. Finally sending "job done" when complete

The Digger generates ore (JTUs) during execution, and this mock printer
just provides the lifecycle events to keep the Digger engaged.

Usage:
    python mock_printer.py --contract-id contract-001 --duration 30
"""

import argparse
import requests
import time
import json
from datetime import datetime
from typing import Dict, Any

# ============================================================================
# Configuration
# ============================================================================

DIGGER_BASE_URL = "http://localhost:3030"  # Digger HTTP port (from docker-compose)

# Fake base64 image (1x1 transparent PNG)
FAKE_PRINTBED_PHOTO = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
)

# ============================================================================
# Color Output Helpers
# ============================================================================

class Colors:
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BLUE = '\033[94m'
    RESET = '\033[0m'

def print_success(msg: str):
    print(f"{Colors.GREEN}✅ {msg}{Colors.RESET}")

def print_info(msg: str):
    print(f"{Colors.BLUE}ℹ️  {msg}{Colors.RESET}")

def print_warning(msg: str):
    print(f"{Colors.YELLOW}⚠️  {msg}{Colors.RESET}")

def print_error(msg: str):
    print(f"{Colors.RED}❌ {msg}{Colors.RESET}")

def print_section(msg: str):
    print(f"\n{Colors.BLUE}{'='*60}")
    print(f"  {msg}")
    print(f"{'='*60}{Colors.RESET}\n")

# ============================================================================
# Mock Printer Logic
# ============================================================================

class MockPrinter:
    def __init__(self, contract_id: str, duration_seconds: int, milestones: int = 5, printer_id: str = "mock-printer-001"):
        self.contract_id = contract_id
        self.duration_seconds = duration_seconds
        self.milestones = milestones
        self.printer_id = printer_id
        self.digger_url = DIGGER_BASE_URL
        
    def create_contract(self, torq: float, robo_stake: float, power_watts: float) -> bool:
        """Create contract in Digger"""
        print_info(f"Creating contract {self.contract_id}...")
        
        try:
            response = requests.post(
                f"{self.digger_url}/contracts/create",
                json={
                    "contract_id": self.contract_id,
                    "torq": torq,
                    "robo_stake": robo_stake,
                    "milestones": self.milestones,
                    "power_watts": power_watts
                },
                timeout=5
            )
            
            if response.status_code == 200:
                data = response.json()
                print_success(f"Contract created: {data['contract_id']}")
                print_info(f"  Ore target: {data['ore_target']} RT")
                return True
            else:
                print_error(f"Failed to create contract: {response.status_code}")
                print_error(f"  Response: {response.text}")
                return False
                
        except Exception as e:
            print_error(f"Error creating contract: {e}")
            return False
    
    def pay_stake(self) -> bool:
        """Pay stake to approve contract"""
        print_info(f"Paying stake for contract {self.contract_id}...")
        
        try:
            response = requests.post(
                f"{self.digger_url}/contracts/stake",
                json={"contract_id": self.contract_id},
                timeout=5
            )
            
            if response.status_code == 200:
                data = response.json()
                print_success(f"Stake paid: {data['approval_status']}")
                return True
            else:
                print_error(f"Failed to pay stake: {response.status_code}")
                return False
                
        except Exception as e:
            print_error(f"Error paying stake: {e}")
            return False
    
    def print_started(self):
        """Notify Digger that printing has started"""
        print_section(f"🖨️  PRINTER STARTED")
        print_info(f"Printer: {self.printer_id}")
        print_info(f"Contract: {self.contract_id}")
        print_info(f"Duration: {self.duration_seconds} seconds")
        print_info(f"Milestones: {self.milestones}")
        
        # Notify Digger that job is starting
        try:
            response = requests.post(
                f"{self.digger_url}/printer/job/start",
                json={
                    "printer_id": self.printer_id,
                    "contract_id": self.contract_id
                },
                timeout=5
            )
            
            if response.status_code == 200:
                data = response.json()
                print_success(f"Digger acknowledged job start")
                print_info(f"  Job ID: {data['job_id']}")
                print_info(f"  Milestones to complete: {data['milestones_total']}")
            else:
                print_warning(f"Failed to notify Digger: {response.status_code}")
        except Exception as e:
            print_warning(f"Error notifying Digger: {e}")
    
    def milestone_check(self, milestone_num: int, elapsed: float) -> Dict[str, Any]:
        """Send milestone update with fake photo to Digger"""
        progress_pct = (elapsed / self.duration_seconds) * 100
        
        print_info(
            f"📸 Milestone {milestone_num}/{self.milestones} "
            f"({progress_pct:.1f}% complete, {elapsed:.1f}s elapsed)"
        )
        
        # Report milestone to Digger
        try:
            response = requests.post(
                f"{self.digger_url}/printer/milestone",
                json={
                    "contract_id": self.contract_id,
                    "milestone_number": milestone_num,
                    "photo_base64": FAKE_PRINTBED_PHOTO,
                    "notes": f"Print progress at {progress_pct:.1f}%, {elapsed:.1f}s elapsed"
                },
                timeout=5
            )
            
            if response.status_code == 200:
                data = response.json()
                print_success(f"  Milestone reported: {data['milestones_completed']}/{data['milestones_total']}")
                return data
            else:
                print_warning(f"  Failed to report milestone: {response.status_code}")
                return {}
                
        except Exception as e:
            print_warning(f"  Error reporting milestone: {e}")
            return {}
        
        # Return fake photo data for local tracking
        return {
            "milestone": milestone_num,
            "elapsed_seconds": elapsed,
            "progress_percent": progress_pct,
            "photo_base64": FAKE_PRINTBED_PHOTO,
            "timestamp": datetime.utcnow().isoformat() + "Z"
        }
    
    def execute_print_job(self) -> bool:
        """Start print job (Digger generates JTUs immediately)"""
        print_info("Starting contract execution on Digger...")
        
        try:
            # Tell Digger to start executing (generates ore/JTUs)
            response = requests.post(
                f"{self.digger_url}/contracts/execute",
                json={
                    "contract_id": self.contract_id,
                    "duration_seconds": self.duration_seconds
                },
                timeout=self.duration_seconds + 10  # Allow time for execution
            )
            
            if response.status_code != 200:
                print_error(f"Failed to execute contract: {response.status_code}")
                return False
            
            data = response.json()
            print_success(f"Contract execution started!")
            print_info(f"  JTUs generated: {data['jtus_generated']}")
            print_info(f"  Ore generated: {data['ore_generated']:.2f} RT")
            print_info(f"  Ore target: {data['ore_target']:.2f} RT")
            print_info(f"  Target reached: {data['target_reached']}")
            
            return True
            
        except Exception as e:
            print_error(f"Error during execution: {e}")
            return False
    
    def simulate_milestones(self):
        """Simulate milestone checks during printing"""
        interval = self.duration_seconds / self.milestones
        
        for i in range(1, self.milestones + 1):
            time.sleep(interval)
            elapsed = i * interval
            milestone_data = self.milestone_check(i, elapsed)
            
            # Milestone is now reported to Digger via HTTP
            if milestone_data:
                print_info(f"  ✓ Milestone {i} acknowledged by Digger")
    
    def job_done(self) -> Dict[str, Any]:
        """Print job completed"""
        print_section("✅ PRINT JOB COMPLETE")
        
        try:
            # Notify Digger job is complete
            response = requests.post(
                f"{self.digger_url}/printer/job/complete",
                json={
                    "contract_id": self.contract_id,
                    "total_duration_seconds": self.duration_seconds
                },
                timeout=5
            )
            
            if response.status_code == 200:
                data = response.json()
                print_success(f"Digger acknowledged job completion!")
                print_info(f"  JTUs generated: {data['jtu_count']}")
                print_info(f"  Ore generated: {data['ore_generated']:.2f} RT")
                return data
            else:
                print_warning(f"Failed to notify completion: {response.status_code}")
        except Exception as e:
            print_warning(f"Error notifying completion: {e}")
        
        # Query final contract status as fallback
        try:
            response = requests.get(
                f"{self.digger_url}/contracts/{self.contract_id}",
                timeout=5
            )
            
            if response.status_code == 200:
                status = response.json()
                
                print_success(f"Contract {self.contract_id} complete!")
                print_info(f"  JTUs generated: {status['jtu_count']}")
                print_info(f"  Ore generated: {status['ore_generated']:.2f} RT")
                print_info(f"  Ore target: {status['ore_target']:.2f} RT")
                print_info(f"  Target reached: {status['target_reached']}")
                print_info(f"  Progress: {status['progress']:.1f}%")
                
                return status
            else:
                print_warning(f"Could not fetch final status: {response.status_code}")
                return {}
                
        except Exception as e:
            print_error(f"Error fetching completion status: {e}")
            return {}
    
    def run_full_cycle(self, torq: float = 100.0, robo_stake: float = 5.0, power_watts: float = 2000.0):
        """Run complete printer lifecycle"""
        print_section(f"Mock Printer - Simulating Physical RT Print Job")
        
        # Step 1: Create contract
        if not self.create_contract(torq, robo_stake, power_watts):
            print_error("Failed to create contract. Exiting.")
            return False
        
        time.sleep(1)
        
        # Step 2: Pay stake
        if not self.pay_stake():
            print_error("Failed to pay stake. Exiting.")
            return False
        
        time.sleep(1)
        
        # Step 3: Start printing
        self.print_started()
        
        # Step 4: Execute contract (Digger generates ore)
        if not self.execute_print_job():
            print_error("Failed to execute print job. Exiting.")
            return False
        
        # Step 5: Simulate milestones while "printing"
        self.simulate_milestones()
        
        # Step 6: Job done
        self.job_done()
        
        print_section("🎉 Mock Printer Cycle Complete!")
        return True

# ============================================================================
# CLI
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Mock Printer - Simulate physical RT printing for Digger testing"
    )
    
    parser.add_argument(
        "--contract-id",
        type=str,
        default=f"mock-contract-{int(time.time())}",
        help="Contract ID (default: auto-generated)"
    )
    
    parser.add_argument(
        "--duration",
        type=int,
        default=30,
        help="Print duration in seconds (default: 30)"
    )
    
    parser.add_argument(
        "--milestones",
        type=int,
        default=5,
        help="Number of milestone checks (default: 5)"
    )
    
    parser.add_argument(
        "--torq",
        type=float,
        default=100.0,
        help="Torq ratio (default: 100.0)"
    )
    
    parser.add_argument(
        "--robo-stake",
        type=float,
        default=5.0,
        help="RoboStake amount in RT (default: 5.0)"
    )
    
    parser.add_argument(
        "--power",
        type=float,
        default=2000.0,
        help="Robot power in watts (default: 2000.0)"
    )
    
    parser.add_argument(
        "--digger-url",
        type=str,
        default=DIGGER_BASE_URL,
        help=f"Digger HTTP API URL (default: {DIGGER_BASE_URL})"
    )
    
    args = parser.parse_args()
    
    # Create and run mock printer (pass digger_url to constructor)
    printer = MockPrinter(
        contract_id=args.contract_id,
        duration_seconds=args.duration,
        milestones=args.milestones
    )
    
    # Override digger_url if provided
    if args.digger_url != DIGGER_BASE_URL:
        printer.digger_url = args.digger_url
    
    success = printer.run_full_cycle(
        torq=args.torq,
        robo_stake=args.robo_stake,
        power_watts=args.power
    )
    
    exit(0 if success else 1)

if __name__ == "__main__":
    main()
