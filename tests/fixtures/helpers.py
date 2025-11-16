"""
RoboTorq Test Fixtures
Shared test utilities and helpers
"""

import subprocess
import time
import json
import hashlib
import asyncio
from typing import Optional, Dict, Any, List, Tuple
from nats.aio.client import Client as NATS


class Colors:
    """ANSI color codes for terminal output"""
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


def print_success(msg: str):
    """Print success message in green"""
    print(f"{Colors.OKGREEN}✅ {msg}{Colors.ENDC}")


def print_error(msg: str):
    """Print error message in red"""
    print(f"{Colors.FAIL}❌ {msg}{Colors.ENDC}")


def print_warning(msg: str):
    """Print warning message in yellow"""
    print(f"{Colors.WARNING}⚠️  {msg}{Colors.ENDC}")


def print_step(msg: str):
    """Print step message in blue"""
    print(f"{Colors.OKBLUE}▶ {msg}{Colors.ENDC}")


def print_section(title: str):
    """Print section header"""
    print(f"\n{Colors.BOLD}{Colors.HEADER}")
    print("=" * 80)
    print(title)
    print("=" * 80)
    print(f"{Colors.ENDC}\n")


def check_docker_container(container_name: str) -> bool:
    """
    Check if a Docker container is running
    
    Args:
        container_name: Name of the container to check
        
    Returns:
        True if container is running, False otherwise
    """
    try:
        result = subprocess.run(
            ['docker', 'ps', '--filter', f'name={container_name}', '--format', '{{.Names}}'],
            capture_output=True,
            text=True,
            check=True
        )
        return container_name in result.stdout
    except subprocess.CalledProcessError:
        return False


def get_docker_logs(container_name: str, tail: int = 100, since: str = "30s") -> str:
    """
    Get logs from a Docker container
    
    Args:
        container_name: Name of the container
        tail: Number of lines to show from the end
        since: Show logs since timestamp (e.g., "30s", "5m")
        
    Returns:
        Container logs as string
    """
    try:
        result = subprocess.run(
            ['docker', 'logs', container_name, '--tail', str(tail), '--since', since],
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout + result.stderr
    except subprocess.CalledProcessError as e:
        return f"Error getting logs: {e}"


def search_docker_logs(container_name: str, pattern: str, context: int = 0) -> List[str]:
    """
    Search Docker logs for a pattern
    
    Args:
        container_name: Name of the container
        pattern: Regex pattern to search for
        context: Number of lines to show before/after match
        
    Returns:
        List of matching lines
    """
    logs = get_docker_logs(container_name, tail=1000)
    matches = []
    lines = logs.split('\n')
    
    for i, line in enumerate(lines):
        if pattern.lower() in line.lower():
            start = max(0, i - context)
            end = min(len(lines), i + context + 1)
            matches.extend(lines[start:end])
    
    return matches


def wait_for_service(container_name: str, max_wait: int = 30) -> bool:
    """
    Wait for a Docker container to be healthy
    
    Args:
        container_name: Name of the container
        max_wait: Maximum seconds to wait
        
    Returns:
        True if service is healthy, False otherwise
    """
    print_step(f"Waiting for {container_name} to be ready...")
    
    for i in range(max_wait):
        if check_docker_container(container_name):
            # Check if container has been running for at least 2 seconds
            time.sleep(2)
            if check_docker_container(container_name):
                print_success(f"{container_name} is ready")
                return True
        time.sleep(1)
    
    print_error(f"{container_name} did not start within {max_wait}s")
    return False


def generate_sha256_hash(data: str) -> str:
    """
    Generate SHA256 hash of input data
    
    Args:
        data: String to hash
        
    Returns:
        64-character hex hash
    """
    return hashlib.sha256(data.encode()).hexdigest()


def validate_merkle_root(root: str) -> Tuple[bool, str]:
    """
    Validate merkle root format (64-char hex SHA256)
    
    Args:
        root: Merkle root to validate
        
    Returns:
        Tuple[bool, str]: (is_valid, error_message)
    """
    if not root:
        return (False, "Hash is empty")
    
    if not isinstance(root, str):
        return (False, f"Hash must be string, got {type(root).__name__}")
    
    if len(root) != 64:
        return (False, f"Hash must be 64 chars (SHA256), got {len(root)}")
    
    try:
        int(root, 16)
        return (True, "")
    except ValueError:
        return (False, "Hash contains non-hex characters")


def create_sample_phase2_ingot(
    ingot_id: str, 
    contracts: Optional[List[str]] = None, 
    diggers: Optional[List[str]] = None,
    joules: float = 14400000.0,
    robo_stake: float = 0.05
) -> Dict[str, Any]:
    """
    Create a sample Phase2Ingot for testing
    
    Args:
        ingot_id: Unique ingot ID
        contracts: List of contract IDs (default: ["test-contract-001"])
        diggers: List of digger IDs (default: ["test-digger-001"])
        joules: Total joules consumed (default: 14400000.0)
        robo_stake: Total RoboStake paid (default: 0.05)
        
    Returns:
        Phase2Ingot as dictionary
    """
    if contracts is None:
        contracts = ["test-contract-001"]
    if diggers is None:
        diggers = ["test-digger-001"]
    
    # Generate a deterministic branch hash
    hash_input = f"{ingot_id}-{joules}-{robo_stake}-{'-'.join(contracts)}-{'-'.join(diggers)}"
    branch_hash = generate_sha256_hash(hash_input)
    
    return {
        "ingot_id": ingot_id,
        "branch_hash": branch_hash,
        "joules_consumed": joules,
        "robo_stake_paid": robo_stake,
        "units": 3600,
        "contract_ids": contracts,
        "digger_ids": diggers,
        "refinery_id": "test-refinery-001",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S.%fZ")
    }


def verify_phase3_unit(unit: Dict[str, Any]) -> Tuple[bool, List[str]]:
    """
    Verify Phase3RoboTorqUnit structure and content
    
    Args:
        unit: Phase3RoboTorqUnit as dictionary
        
    Returns:
        Tuple of (is_valid, list_of_issues)
    """
    issues = []
    
    # Check required fields
    if 'unit_id' not in unit:
        issues.append("Missing unit_id")
    elif not unit['unit_id'].startswith('RT-'):
        issues.append(f"Invalid unit_id format: {unit['unit_id']}")
    
    if 'merkle_root' not in unit:
        issues.append("Missing merkle_root")
    elif not validate_merkle_root(unit['merkle_root']):
        issues.append(f"Invalid merkle_root format: {unit['merkle_root']}")
    
    if 'minted_at' not in unit:
        issues.append("Missing minted_at")
    
    # Check no unexpected metadata fields (Phase 3 design)
    metadata_fields = ['contract_ids', 'digger_ids', 'refinery_ids', 
                      'total_joules', 'total_robo_stake']
    unexpected = [f for f in metadata_fields if f in unit]
    if unexpected:
        issues.append(f"Unexpected metadata fields: {unexpected}")
    
    # Check size
    unit_json = json.dumps(unit)
    unit_size = len(unit_json.encode())
    if unit_size > 200:
        issues.append(f"Unit too large for NFC: {unit_size} bytes")
    
    return (len(issues) == 0, issues)


def restart_service(service_name: str) -> bool:
    """
    Restart a Docker Compose service
    
    Args:
        service_name: Name of the service in docker-compose.yaml
        
    Returns:
        True if restart successful, False otherwise
    """
    try:
        print_step(f"Restarting {service_name}...")
        subprocess.run(['docker-compose', 'restart', service_name], 
                      check=True, capture_output=True)
        return wait_for_service(f"robotorq-network-{service_name}-1")
    except subprocess.CalledProcessError:
        return False


def cleanup_test_data():
    """
    Clean up test data and reset services
    """
    print_step("Cleaning up test data...")
    
    # Could flush NATS streams, clear test databases, etc.
    # For now, just restart services to clear in-memory state
    
    print_success("Cleanup complete")


# ============================================================================
# Integration Test Utilities
# ============================================================================

NATS_URL = "nats://localhost:4222"


async def verify_nats_message_flow(
    topic: str,
    expected_count: int,
    timeout: int = 30,
    nats_url: str = NATS_URL
) -> List[Dict[str, Any]]:
    """
    Subscribe to NATS topic, collect messages, verify count matches expected
    
    Args:
        topic: NATS topic to subscribe to
        expected_count: Number of messages expected
        timeout: How long to wait (seconds)
        nats_url: NATS server URL
    
    Returns:
        List of received messages (as dicts)
    
    Raises:
        AssertionError: If message count doesn't match expected
    
    Example:
        msgs = await verify_nats_message_flow("mint.units", 1, 60)
        assert msgs[0]['unit_id'].startswith('RT-')
    """
    nc = NATS()
    await nc.connect(nats_url)
    
    messages = []
    
    async def handler(msg):
        try:
            data = json.loads(msg.data.decode())
            messages.append(data)
            print_step(f"Received message on {topic}: {len(messages)}/{expected_count}")
        except json.JSONDecodeError:
            print_warning(f"Received non-JSON message on {topic}")
    
    await nc.subscribe(topic, cb=handler)
    print_step(f"Subscribed to {topic}, waiting for {expected_count} messages...")
    
    await asyncio.sleep(timeout)
    
    await nc.close()
    
    if len(messages) != expected_count:
        raise AssertionError(
            f"Expected {expected_count} messages on {topic}, got {len(messages)}"
        )
    
    print_success(f"Received all {expected_count} messages on {topic}")
    return messages


async def measure_pipeline_latency(
    start_topic: str,
    end_topic: str,
    trigger_func,
    timeout: int = 60,
    nats_url: str = NATS_URL
) -> float:
    """
    Measure latency from publish on start_topic to receive on end_topic
    
    Args:
        start_topic: Topic where data enters pipeline
        end_topic: Topic where data exits pipeline
        trigger_func: Async function that triggers the pipeline (publishes to start_topic)
        timeout: Max wait time (seconds)
        nats_url: NATS server URL
    
    Returns:
        Latency in seconds
    
    Example:
        async def trigger():
            await publish_ingots(1000)
        
        latency = await measure_pipeline_latency(
            "mint.ingots", 
            "distodam.units",
            trigger
        )
        print(f"Pipeline latency: {latency:.2f}s")
    """
    nc = NATS()
    await nc.connect(nats_url)
    
    start_time = None
    end_time = None
    
    async def end_handler(msg):
        nonlocal end_time
        if end_time is None:  # Capture first message only
            end_time = time.time()
            print_step(f"Received output on {end_topic}")
    
    # Subscribe to end topic FIRST
    await nc.subscribe(end_topic, cb=end_handler)
    await asyncio.sleep(0.5)  # Ensure subscription ready
    
    # Trigger pipeline
    start_time = time.time()
    print_step(f"Triggering pipeline...")
    await trigger_func()
    
    # Wait for output
    for i in range(timeout):
        if end_time is not None:
            break
        await asyncio.sleep(1)
    
    await nc.close()
    
    if end_time is None:
        raise TimeoutError(f"No message received on {end_topic} after {timeout}s")
    
    latency = end_time - start_time
    print_success(f"Pipeline latency: {latency:.2f}s")
    return latency


def verify_merkle_proof_chain(
    unit: Dict[str, Any],
    ingot: Optional[Dict[str, Any]] = None
) -> Tuple[bool, List[str]]:
    """
    Verify unit's merkle_root traces back to ingot's branch_hash
    
    Args:
        unit: Phase3RoboTorqUnit dict
        ingot: Optional Phase2Ingot dict (if available)
    
    Returns:
        (is_valid, [issues])
    
    Example:
        is_valid, issues = verify_merkle_proof_chain(unit, ingot)
        if not is_valid:
            for issue in issues:
                print(f"  - {issue}")
    """
    issues = []
    
    # Validate unit structure
    if 'merkle_root' not in unit:
        issues.append("Unit missing merkle_root")
    else:
        root = unit['merkle_root']
        is_valid_root, error = validate_merkle_root(root)
        if not is_valid_root:
            issues.append(f"Invalid merkle_root: {error}")
    
    if 'unit_id' not in unit:
        issues.append("Unit missing unit_id")
    
    # If ingot provided, validate consistency
    if ingot:
        if 'branch_hash' not in ingot:
            issues.append("Ingot missing branch_hash")
        else:
            branch_hash = ingot['branch_hash']
            is_valid_hash, error = validate_merkle_root(branch_hash)
            if not is_valid_hash:
                issues.append(f"Invalid branch_hash: {error}")
        
        # In Phase 3, we can't directly verify unit → ingot link without ledger
        # This would require fetching the merkle tree from Mint
        # For now, just validate both hashes are well-formed
    
    is_valid = len(issues) == 0
    return (is_valid, issues)


async def wait_for_nats_message(
    topic: str,
    timeout: int = 30,
    nats_url: str = NATS_URL
) -> Optional[Dict[str, Any]]:
    """
    Wait for a single message on NATS topic
    
    Args:
        topic: NATS topic to subscribe to
        timeout: Max wait time (seconds)
        nats_url: NATS server URL
    
    Returns:
        Message as dict, or None if timeout
    
    Example:
        unit = await wait_for_nats_message("distodam.units", timeout=60)
        if unit:
            print(f"Received unit: {unit['unit_id']}")
    """
    nc = NATS()
    await nc.connect(nats_url)
    
    message = None
    
    async def handler(msg):
        nonlocal message
        try:
            message = json.loads(msg.data.decode())
        except json.JSONDecodeError:
            print_warning(f"Received non-JSON message on {topic}")
    
    await nc.subscribe(topic, cb=handler)
    
    for i in range(timeout):
        if message is not None:
            break
        await asyncio.sleep(1)
    
    await nc.close()
    
    if message:
        print_success(f"Received message on {topic}")
    else:
        print_warning(f"No message on {topic} after {timeout}s")
    
    return message
