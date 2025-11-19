# Pipeline Logs Scripts

Quick access to container logs for each service in the RoboTorq pipeline.

## Usage

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
