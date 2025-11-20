# Mock Printer Testing Guide

Since the real Ender 3 V3 KE Moonraker API is blocked by firewall (port 7125), we've implemented **mock mode** for testing the complete RoboTorq printer integration.

## Quick Start

### 1. Configure Mock Mode

Edit `config.yaml`:

```yaml
mock_mode: true  # Enable mock mode (no real printer needed)
```

### 2. Build & Run

```bash
# In Docker (recommended - has liboqs)
cd src/printer
docker build -t printer:latest .
docker run --rm \
  --network host \
  -v ${PWD}/config.yaml:/app/config.yaml \
  printer:latest

# Or locally (if you have liboqs installed)
cargo run
```

### 3. Control Mock Printer

The mock printer exposes a control API on port **9092**:

#### Check Status
```bash
curl http://localhost:9092/status
```

Response:
```json
{
  "state": "idle",
  "is_printing": false,
  "duration_secs": 0.0
}
```

#### Start Print
```bash
curl -X POST http://localhost:9092/start
```

This simulates pressing PRINT on the physical printer. The printer service will:
1. Detect state change (idle → printing)
2. Publish `printer.contract_started` to NATS
3. Begin tracking capacity (350W × duration)

#### Complete Print
```bash
curl -X POST http://localhost:9092/complete
```

This simulates print completion. The printer service will:
1. Detect state change (printing → idle)
2. Calculate total capacity used (watts × hours)
3. Publish `printer.contract_completed` to NATS with capacity ore

## Testing Flow

1. **Start Services**
   ```bash
   docker-compose up -d nats digger
   docker run --network robotorq-network printer:latest
   ```

2. **Register Printer** (automatic on startup)
   - Printer sends registration to `printer.register`
   - Digger issues certificate
   - Certificate saved to `data/certificate.json`

3. **Assign Contract** (via Digger API)
   ```bash
   # Create contract first
   curl -X POST http://localhost:9000/contract \
     -H "Content-Type: application/json" \
     -d '{"description": "Test print job"}'

   # Assign to printer
   curl -X POST http://localhost:9000/contract/{id}/assign_printer \
     -H "Content-Type: application/json" \
     -d '{"printer_id": "ender3-v3-ke-001"}'
   ```

4. **Simulate Print**
   ```bash
   # Start print
   curl -X POST http://localhost:9092/start

   # Wait some time (simulate print duration)
   sleep 30

   # Complete print
   curl -X POST http://localhost:9092/complete
   ```

5. **Verify Ore Generated**
   - Check Digger logs for `printer.contract_completed` event
   - Verify JouleTorqOre created with capacity value
   - Ore should show: `350W × 30s = 2.92 watt-hours`

## Switching to Real Printer

When ready to connect the physical Ender 3 V3 KE:

### 1. Fix Firewall/Network

Option A: **Enable Moonraker external access**
```bash
# SSH into printer
ssh pi@192.168.12.182

# Edit Moonraker config
nano ~/printer_data/config/moonraker.conf

# Ensure these settings:
[server]
host: 0.0.0.0  # Listen on all interfaces
port: 7125

[authorization]
cors_domains:
    *  # Allow all origins (or restrict to your network)

# Restart Moonraker
sudo systemctl restart moonraker
```

Option B: **Run printer service ON the printer's Raspberry Pi**
- No firewall issues (localhost access)
- Service runs 24/7 alongside printer

### 2. Update Config

```yaml
mock_mode: false  # Disable mock mode
klipper_url: "http://192.168.12.182:7125"  # Or http://localhost:7125 if running on Pi
```

### 3. Deploy

```bash
# On printer's Raspberry Pi
scp -r src/printer pi@192.168.12.182:~/robotorq/
ssh pi@192.168.12.182
cd ~/robotorq/printer
cargo build --release
./target/release/printer
```

## Mock vs Real Comparison

| Feature | Mock Mode | Real Printer |
|---------|-----------|--------------|
| API Control | HTTP POST to /start, /complete | Automatic via Moonraker polling |
| State Detection | Manual trigger | Automatic (polls every 5s) |
| Print Duration | Measured between start/complete | Actual print time from Klipper |
| Network Required | No (local only) | Yes (Moonraker API access) |
| Hardware Required | None | Ender 3 V3 KE with Klipper |
| Use Case | Testing, development | Production, real work |

## Troubleshooting

### Mock API not responding
```bash
# Check if running
curl http://localhost:9092/status

# Check logs
docker logs <container-id>

# Verify port not in use
netstat -an | grep 9092
```

### Mock prints not detected by Digger
- Verify NATS connection: `docker logs robotorq-network-nats-1`
- Check Digger subscriptions: `docker logs robotorq-network-digger-1 | grep printer`
- Ensure printer registered: Check for certificate in `data/certificate.json`

### Switching between mock and real
- Stop service: `Ctrl+C` or `docker stop`
- Edit `config.yaml`: Change `mock_mode`
- Restart service

## Metrics

Both mock and real mode expose Prometheus metrics on port **9091**:

```bash
curl http://localhost:9091/metrics | grep printer_
```

Key metrics:
- `printer_heartbeats_total` - Total heartbeats sent
- `printer_prints_started_total` - Total prints started
- `printer_prints_completed_total` - Total prints completed
- `printer_capacity_hours_total` - Total capacity tracked (watt-hours)

## Next Steps

1. **Implement Digger handlers** for printer events
2. **Test full flow** with mock mode
3. **Fix Moonraker access** for real printer
4. **Deploy to Pi** for production use
