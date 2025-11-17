# Test Migration Guide

This guide shows how to refactor existing E2E tests to use the new test fixtures.

## Before: Inline Print Statements

```python
#!/usr/bin/env python3
import asyncio
from nats.aio.client import Client as NATS

async def main():
    print("=" * 80)
    print("My E2E Test")
    print("=" * 80)
    
    print("\n🔌 Connecting to NATS...")
    nc = NATS()
    try:
        await nc.connect("nats://localhost:4222")
        print("✅ Connected to NATS")
    except Exception as e:
        print(f"❌ Failed to connect: {e}")
        print("💡 Make sure NATS is running")
        return
    
    # ... test logic ...
    
    print("\n📊 TEST RESULTS")
    print(f"✅ Test passed!")
```

## After: Using Test Fixtures

```python
#!/usr/bin/env python3
import asyncio
import sys
from nats.aio.client import Client as NATS

# Import test fixtures
sys.path.append('tests/fixtures')
from helpers import *

async def main():
    print_section("My E2E Test")
    
    # Check prerequisites
    if not check_docker_container("robotorq-network-nats-1"):
        print_error("NATS not running")
        print_step("Start with: docker-compose up -d nats")
        return False
    
    print_step("Connecting to NATS...")
    nc = NATS()
    try:
        await nc.connect("nats://localhost:4222")
        print_success("Connected to NATS")
    except Exception as e:
        print_error(f"Failed to connect: {e}")
        return False
    
    # ... test logic ...
    
    print_section("TEST RESULTS")
    print_success("Test passed!")
    return True

if __name__ == "__main__":
    success = asyncio.run(main())
    exit(0 if success else 1)
```

## Available Helper Functions

### Console Output
- `print_success(msg)` - ✅ Green success message
- `print_error(msg)` - ❌ Red error message
- `print_warning(msg)` - ⚠️ Yellow warning
- `print_step(msg)` - ▶ Blue info step
- `print_section(title)` - Section header with ===

### Docker Helpers
- `check_docker_container(name)` - Returns True if container running
- `get_docker_logs(name, since="60s")` - Get container logs
- `search_docker_logs(name, pattern, since="60s")` - Search logs for pattern
- `wait_for_service(name, timeout=30)` - Wait for container healthy

### Validators
- `validate_merkle_root(hash)` - Check if valid 64-char hex
- `verify_phase3_unit(unit)` - Validate Phase3RoboTorqUnit structure

### Sample Data
- `create_sample_phase2_ingot(id, contracts=[], diggers=[])` - Generate test ingot

## Migration Checklist

For each E2E test in `tests/e2e/`:

- [ ] Add `import sys; sys.path.append('tests/fixtures'); from helpers import *`
- [ ] Replace `print("=...")` with `print_section(title)`
- [ ] Replace `print("✅...")` with `print_success(msg)`
- [ ] Replace `print("❌...")` with `print_error(msg)`
- [ ] Replace `print("⚠️...")` with `print_warning(msg)`
- [ ] Replace `print("💡...")` or info prints with `print_step(msg)`
- [ ] Add Docker checks: `check_docker_container()` before connecting
- [ ] Add return values: `return True/False` for success/failure
- [ ] Add exit code: `exit(0 if success else 1)` at end

## Example: Docker Log Verification

### Before
```python
print("Check logs manually:")
print("docker logs robotorq-network-mint-1 | grep 'ingot assembled'")
```

### After
```python
if check_docker_container("robotorq-network-mint-1"):
    logs = get_docker_logs("robotorq-network-mint-1", since="60s")
    if "ingot assembled" in logs:
        print_success("Ingot found in logs!")
    else:
        print_warning("Ingot not found in recent logs")
else:
    print_error("Mint container not running")
```

## Example: Service Health Check

### Before
```python
try:
    await nc.connect(NATS_URL)
except Exception as e:
    print(f"Failed to connect: {e}")
    return
```

### After
```python
if not wait_for_service("robotorq-network-nats-1", timeout=30):
    print_error("NATS failed to start")
    print_step("Check logs: docker logs robotorq-network-nats-1")
    return False

try:
    await nc.connect(NATS_URL)
    print_success("Connected to NATS")
except Exception as e:
    print_error(f"Failed to connect: {e}")
    return False
```

## Running Migrated Tests

```bash
# Single test
python tests/e2e/phase3_complete.py

# All E2E tests
pytest tests/e2e/ -v

# With output capture disabled (see colors)
pytest tests/e2e/ -v -s
```

## Benefits of Using Fixtures

1. **Consistency**: All tests use same colored output format
2. **Readability**: Helper names are clear (`print_success` vs `print("✅...")`)
3. **Reusability**: Docker checks, log searches work across all tests
4. **Maintainability**: Change output format once in `helpers.py`
5. **Debugging**: Built-in Docker integration for troubleshooting
6. **Validation**: Shared validators ensure consistent test criteria
