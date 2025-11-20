# RoboTorq Printer Service

**Physical robot agent** for connecting 3D printers to the RoboTorq network.

## Overview

The Printer Service is an edge agent that:
- Registers with Digger to get a bonded certificate
- Monitors printer state (idle/printing) via Klipper/Moonraker
- Reports capacity usage based on certified power rating
- Generates verifiable work proofs

## Architecture

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

### Certificate Setup

On first run, the printer will:
1. Generate Falcon-1024 keypair (if needed)
2. Request bonded certificate from Digger
3. Save certificate to `./data/{printer_id}_certificate.json`

Certificate includes:
- Printer ID and model
- Certified power rating (watts)
- Mint signature (proof of bonding)
- Validity period

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
printer_certificate_valid (gauge: 0 or 1)
```

## Security

### Certificate Validation

- Certificate signed by Mint (SPHINCS+)
- Includes printer public key
- Verified on every heartbeat
- Stored locally (not transmitted)

### Attack Vectors

- **Spoofed printer**: Prevented by certificate binding
- **Inflated capacity**: Digger validates model specs
- **False reporting**: Klipper logs provide audit trail
- **Certificate theft**: Requires physical access to edge device

## Development

### Mock Mode (No Printer)

```bash
# Run without Klipper
MOCK_PRINTER=true cargo run

# Simulates printing every 60 seconds
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

- [x] Basic status reporting
- [x] Certificate management
- [x] Capacity tracking
- [ ] Job queue (accept work from Digger)
- [ ] Milestone reporting (layer progress)
- [ ] Multi-printer support (one service, N printers)
- [ ] OctoPrint integration
- [ ] Direct Klipper socket (no Moonraker)

## License

See root LICENSE file
