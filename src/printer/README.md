# RoboTorq Printer Service

**Physical robot agent** for connecting 3D printers to the RoboTorq network.

## Status

✅ **COMPLETE**: End-to-end ore generation working (Nov 20, 2025)

- Mock mode fully operational with contract execution
- NATS lifecycle events publishing correctly
- Digger executes contracts and generates JTUs
- Printer registry persists across restarts
- Contract assignment and state tracking working

## Overview

The Printer Service is an edge agent that:
- Connects directly to Klipper/Moonraker (no certificate in MVP)
- Monitors printer state (idle/printing) via Klipper/Moonraker
- Listens for contract assignments via NATS
- Publishes lifecycle events (contract_started, contract_completed)
- Enables Digger to execute contracts and generate ore

## Architecture

### Current Implementation (Mock Mode)

```
┌─────────────────────────────────────────┐
│  Mock API (Port 9092)                   │
│  - Simulates Klipper is_printing()      │
│  - Accepts contract assignment          │
│  - Shared state with PrinterService     │
└──────────────┬──────────────────────────┘
               │ Shared Arc<MockKlipperClient>
┌──────────────▼──────────────────────────┐
│  PrinterService (Heartbeat: 10s)        │
│  - Monitors is_printing() state         │
│  - Detects state changes                │
│  - Publishes NATS lifecycle events      │
└──────────────┬──────────────────────────┘
               │ NATS
┌──────────────▼──────────────────────────┐
│     Digger                               │
│  - Assigns contracts via NATS           │
│  - Listens for lifecycle events         │
│  - Executes contracts on completion     │
│  - Generates JTUs and ore                │
└──────────────────────────────────────────┘
```

**Key Implementation Details:**
- Mock API and PrinterService share `Arc<MockKlipperClient>` to detect state changes
- Contract assignments stored in `Arc<RwLock<MockContractState>>`
- NATS listener started **after** mock state is set (critical initialization order)
- Heartbeat checks printing state every 10 seconds
- State changes trigger NATS events (contract_started, contract_completed)

### Future: Real Klipper Integration

```
┌─────────────────────┐
│  Ender 3 V3 KE      │
│  - Klipper firmware │
│  - Moonraker API    │
└──────────┬──────────┘
           │ HTTP
┌──────────▼──────────┐
│  Printer Service    │
│  - Status monitor   │
│  - Certificate mgmt │
│  - NATS client      │
└──────────┬──────────┘
           │ NATS
┌──────────▼──────────┐
│     Digger          │
│  - Issues certs     │
│  - Tracks printers  │
│  - Aggregates work  │
└─────────────────────┘
```

## Setup

### Prerequisites

- Rust 1.75+
- Ender 3 V3 KE with Klipper/Moonraker
- NATS server running
- Digger service running

### Installation

```bash
# Build
cargo build --release

# Run
./target/release/printer

# Or with custom config
./target/release/printer --config custom-config.yaml
```

### Configuration

Edit `config.yaml`:

```yaml
printer_id: "ender3-v3-ke-001"      # Unique printer identifier
printer_model: "Ender3V3KE"         # Model for certification
rated_watts: 350                    # Certified power capacity
nats_url: "nats://localhost:4222"
klipper_url: "http://localhost:7125"
```

### Certificate Setup (Removed in MVP)

The initial MVP intentionally omits certificate generation and signing to reduce complexity.
Future versions will restore a lightweight bonding + signature flow once physical validation
and power measurement are integrated.

## NATS Event Flow

### Registration (Removed in MVP)
No network registration or bonding occurs. Printer simply publishes status and lifecycle
events. Future implementation will add a registration handshake.

### Contract Assignment

1. User calls Digger API: `POST /printer/assign`
2. Digger publishes: `digger.contract.assigned.{printer_id}`
   ```json
   {
     "contract_id": "test-contract-001",
     "printer_id": "test-printer-001"
   }
   ```
3. Printer receives assignment, stores in `mock_contract_state`

### Contract Execution

1. Print starts (user calls mock API: `POST http://localhost:9092/start?contract_id=...`)
2. PrinterService heartbeat detects `is_printing=true`
3. Printer publishes: `printer.contract_started`
   ```json
   {
     "contract_id": "test-contract-001",
     "printer_id": "test-printer-001",
     "timestamp": "2025-11-20T22:00:00Z"
   }
   ```

4. Print completes
5. PrinterService heartbeat detects `is_printing=false`
6. Printer publishes: `printer.contract_completed`
   ```json
   {
     "contract_id": "test-contract-001",
     "printer_id": "test-printer-001",
     "watt_hours": 0.97
   }
   ```

7. Digger receives completion event
8. Digger executes contract (generates JTUs and ore)
   ```
   ✨ Contract executed: 10000 JTUs generated, 100.00 RT ore (20.0% of target)
   ```

## Operation

### Status Reporting

Printer sends heartbeat every 10 seconds:

```json
{
  "printer_id": "ender3-v3-ke-001",
  "is_printing": true,
  "rated_watts": 350,
  "timestamp": "2025-11-19T10:30:00Z"
}
```

### Print Completion

When print finishes, reports capacity usage:

```json
{
  "printer_id": "ender3-v3-ke-001",
  "duration_hours": 2.5,
  "capacity_kwh": 0.875,
  "rated_watts": 350
}
```

**RoboStake calculation:**
```
Capacity = 350W × 2.5hr = 0.875 kWh
RoboStake = 0.875 kWh × RT_rate
```

## Klipper Integration

### Moonraker API Queries

```bash
# Get printer status
curl http://localhost:7125/printer/objects/query?print_stats

# Response
{
  "result": {
    "status": {
      "print_stats": {
        "state": "printing"  # standby, printing, paused, complete
      }
    }
  }
}
```

### Supported States

- `standby` → Idle (no RoboStake consumed)
- `printing` → Active (RoboStake accumulating)
- `paused` → Active (still consuming capacity)
- `complete` → Generates completion report
- `cancelled` → Generates completion report
- `error` → Logs error, no completion

## Metrics

Prometheus metrics on port 9091:

```
printer_heartbeats_sent_total
printer_prints_completed_total
printer_capacity_kwh_total
printer_uptime_seconds
(removed: printer_certificate_valid)
```

## Security

### Certificate Validation (Deferred)
Will be added after power-proof and bonding workflow are finalized.

### Attack Vectors

- **Spoofed printer**: Prevented by certificate binding
- **Inflated capacity**: Digger validates model specs
- **False reporting**: Klipper logs provide audit trail
- **Certificate theft**: Requires physical access to edge device

## Development

### Mock Mode (Current Implementation)

**Fully working end-to-end flow!**

```bash
# 1. Start services
docker-compose up -d

# 2. Run test script
python scripts/test_printer_flow.py
```

The test script:
1. Registers printer with Digger
2. Creates contract with ore target
3. Funds contract with RoboStake
4. Assigns contract to printer (via NATS)
5. Starts mock print (15 seconds)
6. Printer detects state change and publishes events
7. Digger executes contract and generates ore

**Expected output:**
```
✅ All 7 steps passed!
✅ Contract executed: 10000 JTUs generated, 100.00 RT ore (20.0% of target)
```

**Mock API endpoints:**
```bash
# Start print with contract
curl -X POST "http://localhost:9092/start?contract_id=test-contract-001&duration_secs=15"

# Check if printing
curl http://localhost:9092/is_printing

# Stop print
curl -X POST http://localhost:9092/stop
```

### Testing

```bash
cargo test
cargo test --test integration_test
```

### Cross-Compilation (Raspberry Pi)

```bash
# Install cross-compilation target
rustup target add armv7-unknown-linux-gnueabihf

# Build for Pi
cargo build --release --target armv7-unknown-linux-gnueabihf

# Deploy
scp target/armv7-unknown-linux-gnueabihf/release/printer pi@printer.local:/home/pi/
```

## Deployment

### Raspberry Pi Setup

```bash
# 1. Copy binary to Pi
scp target/release/printer pi@printer.local:/home/pi/robotorq/

# 2. Copy config
scp config.yaml pi@printer.local:/home/pi/robotorq/

# 3. Create systemd service
sudo nano /etc/systemd/system/robotorq-printer.service
```

```ini
[Unit]
Description=RoboTorq Printer Service
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/robotorq
ExecStart=/home/pi/robotorq/printer
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
# 4. Enable and start
sudo systemctl enable robotorq-printer
sudo systemctl start robotorq-printer

# 5. Check status
sudo systemctl status robotorq-printer
journalctl -u robotorq-printer -f
```

## Troubleshooting

### Cannot connect to Klipper

```bash
# Check Moonraker is running
curl http://localhost:7125/printer/info

# Check firewall
sudo ufw allow 7125/tcp
```

### Certificate not received

```bash
# Check NATS connectivity
nats sub "printer.*.certificate"

# Check Digger logs
docker logs robotorq-digger
```

### Metrics not appearing

```bash
# Check Prometheus scraping
curl http://localhost:9091/metrics
```

## Roadmap

### Completed ✅

- [x] Basic status reporting
- [x] (Removed) Certificate management
- [x] NATS event publishing (contract_started, contract_completed)
- [x] Contract assignment listener
- [x] Shared state between mock API and service
- [x] End-to-end ore generation in mock mode
- [x] Printer registry persistence
- [x] Contract execution integration with Digger

### Next Steps: Real Printer Integration

**Option 1: Direct Integration (No Pi)**
- Connect PrinterService directly to Klipper/Moonraker on local network
- Run PrinterService on main Digger machine
- Simpler setup, but requires Digger to be near printer

**Option 2: Raspberry Pi Edge Agent**
- Cross-compile PrinterService for ARM
- Deploy to Raspberry Pi connected to printer
- Pi runs PrinterService as systemd service
- More flexible, printer can be anywhere on network

**Implementation Tasks:**

#### 1. Real Klipper Client
- [ ] Implement `KlipperClient` trait for Moonraker HTTP API
- [ ] Replace `MockKlipperClient` with conditional compilation
- [ ] Add `check_if_printing()` using `/printer/objects/query?print_stats`
- [ ] Handle Moonraker connection errors gracefully
- [ ] Add retry logic for transient failures

#### 2. Print Job Tracking
- [ ] Subscribe to Klipper print events (via websocket or polling)
- [ ] Track print start/stop timestamps accurately
- [ ] Calculate actual watt-hours consumed
- [ ] Report layer progress as milestones (future: proof generation)

#### 3. Power Measurement
- [ ] Integrate with power meter (Shelly Plug, TP-Link Kasa, etc.)
- [ ] Replace fixed `rated_watts` with actual consumption
- [ ] Log power usage throughout print
- [ ] Generate verifiable power consumption proof

#### 4. Configuration Management
- [ ] Auto-detect Klipper URL (mDNS/Avahi)
- [ ] Support multiple printers per service instance
- [ ] Add printer profiles (bed size, nozzle, materials)
- [ ] Environment-based config (dev/staging/prod)

#### 5. Deployment Automation
- [ ] Create Pi SD card image with PrinterService pre-installed
- [ ] Auto-registration on first boot
- [ ] Web UI for initial setup (WiFi, Digger URL)
- [ ] OTA updates for PrinterService binary

#### 6. Security Hardening
- [ ] Certificate rotation (before expiry)
- [ ] Secure storage for private keys (TPM/keyring)
- [ ] Encrypted NATS connections (TLS)
- [ ] Rate limiting for API endpoints

### Decision Points

**Where to run PrinterService?**

| Factor | Direct (No Pi) | Raspberry Pi |
|--------|----------------|--------------|
| Hardware cost | $0 | ~$75 (Pi + case + SD) |
| Network flexibility | Digger must be near printer | Printer anywhere on LAN |
| Latency | Lower (local) | Slightly higher (network) |
| Scalability | One printer per Digger | Many printers per network |
| Recommended for | **Development/testing** | **Production deployment** |

**Power measurement strategy?**

| Option | Accuracy | Cost | Complexity |
|--------|----------|------|------------|
| Fixed rating | Low (assumes 100% usage) | $0 | Simple |
| Smart plug | Medium (whole printer) | ~$25 | Easy (HTTP API) |
| Inline meter | High (DC rails) | ~$50 | Complex (requires wiring) |
| Klipper integration | High (MCU reported) | $0 | Moderate (firmware mod) |

### Recommended Next Steps

1. **Implement real KlipperClient** (2-4 hours)
   - Replace mock with actual Moonraker HTTP calls
   - Test with local Ender 3 V3 KE

2. **Deploy to Raspberry Pi** (4-6 hours)
   - Cross-compile for ARM
   - Create systemd service
   - Test end-to-end on Pi

3. **Add smart plug integration** (2-3 hours)
   - Support Shelly Plug S (local HTTP API)
   - Replace `rated_watts` with actual measurements
   - Log power consumption timeseries

4. **Test multi-hour prints** (8+ hours wall time)
   - Run overnight print job
   - Verify heartbeat reliability
   - Check JTU calculations against actual power usage

5. **Document deployment guide** (2 hours)
   - Step-by-step Pi setup
   - Network configuration
   - Troubleshooting common issues

**Total estimated work: 18-23 hours + testing time**

## License

See root LICENSE file
