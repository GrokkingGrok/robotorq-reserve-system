# Pipeline Logs Scripts

Quick access to container logs for each service in the RoboTorq pipeline.

## Usage

### Basic (streams last 50 lines + follows in real-time)

```bash
# View Digger logs
python scripts/logs/diggerlogs.py

# View Refinery logs
python scripts/logs/refinerylogs.py

# View Mint logs
python scripts/logs/mintlogs.py

# View DistoDam logs
python scripts/logs/distodamlogs.py

# View NATS logs
python scripts/logs/natslogs.py
```

### With Arguments

```bash
# View last 100 lines only (no follow)
python scripts/logs/diggerlogs.py --tail 100

# View last 20 lines
python scripts/logs/mintlogs.py --tail 20

# Follow logs in real-time (skip initial tail, stream continuously)
python scripts/logs/refinerylogs.py -f

# Same as above (long form)
python scripts/logs/distodamlogs.py --follow
```

### Real-World Examples

```bash
# Monitor Mint during batch processing
python scripts/logs/mintlogs.py --follow

# Check recent errors in Refinery (last 30 lines)
python scripts/logs/refinerylogs.py --tail 30

# View Digger startup sequence
python scripts/logs/diggerlogs.py --tail 100

# Follow NATS broker traffic
python scripts/logs/natslogs.py -f

# Combine: view last 200 lines then follow
python scripts/logs/mintlogs.py --tail 200 --follow
```

## Features

- Streams last 50 lines then follows in real-time
- Checks if container is running before streaming
- Press `Ctrl+C` to stop streaming

## Container Names

- Digger: `robotorq-network-digger-1`
- Refinery: `robotorq-network-refinery-1`
- Mint: `robotorq-network-mint-1`
- DistoDam: `robotorq-network-distodam-1`
- NATS: `robotorq-network-nats-1`
