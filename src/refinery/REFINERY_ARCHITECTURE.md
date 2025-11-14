# Refinery Service Architecture

## Overview

The Refinery service is a critical component in the RoboTorq network that:
1. Receives JouleTorqOre from Digger robots via HTTP
2. Accumulates ore into 3600J ingots (TokenTorqIngot)
3. Publishes batches of ingots to Mint via NATS messaging

The service is built with a modular, concurrent architecture using Go's goroutines and channels for high-throughput ore processing.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                       Refinery Service                          │
│                                                                 │
│  HTTP POST /receive-ore                                         │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ OreReceiver  │ ◄── Validates JouleTorqOre structure         │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │QueueManager  │ ◄── Thread-safe buffered channels            │
│  │              │     - JouleQueue (1000 capacity)             │
│  │              │     - RoboQueue (1000 capacity)              │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │IngotAssembler│ ◄── Accumulates to 3600J threshold           │
│  │ (goroutine)  │     Creates TokenTorqIngot                   │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ BatchSender  │ ◄── Interval-based (60s default)             │
│  │ (goroutine)  │     Sends completed ingots                   │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ▼                                                       │
│  ┌──────────────┐                                               │
│  │ MintClient   │ ◄── NATS publisher with retry logic          │
│  │              │     Topic: mint.ingots                       │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ▼                                                       │
│    NATS Broker ──► Mint Service                                │
│                                                                 │
│  Additional Components:                                         │
│  • Health Handler (/health, /metrics)                          │
│  • Config (environment variables)                              │
│  • Metrics (Prometheus)                                        │
│  • Graceful Shutdown (context.Context)                         │
└─────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. OreReceiver (`internal/refinery/ore_receiver.go`)

**Purpose**: HTTP endpoint handler for receiving ore from Digger robots

**Responsibilities**:
- Handles `POST /receive-ore` HTTP requests
- Validates `JouleTorqOre` structure (digger_id, contract_id, joules > 0, etc.)
- Generates hash for each ore
- Enqueues joule and robo stake data separately

**Key Methods**:
- `HTTPHandler(w http.ResponseWriter, r *http.Request)` - HTTP endpoint
- `ReceiveOre(ore *models.JouleTorqOre) error` - Core processing logic

**Error Handling**:
- Returns HTTP 400 for validation errors
- Returns HTTP 429 when queues are full (backpressure)
- Returns HTTP 500 for internal errors

### 2. QueueManager (`internal/refinery/queue_manager.go`)

**Purpose**: Thread-safe queue management for joule and robo stake data

**Responsibilities**:
- Maintains two buffered channels: `jouleQueue` and `roboQueue`
- Provides non-blocking add operations with backpressure detection
- Tracks queue usage metrics

**Key Features**:
- Buffered channels with configurable capacity (default 1000)
- Non-blocking adds with immediate error return when full
- Queue usage percentage tracking for monitoring

**Configuration**:
- `REFINERY_JOULE_QUEUE_SIZE` (default: 1000)
- `REFINERY_ROBO_QUEUE_SIZE` (default: 1000)

### 3. IngotAssembler (`internal/refinery/ingot_assembler.go`)

**Purpose**: Accumulates ore into 3600J ingots

**Responsibilities**:
- Runs as a goroutine consuming from both queues
- Accumulates joules until 3600J threshold is reached
- Creates `TokenTorqIngot` with:
  - Total joules (always 3600J when threshold-triggered)
  - Total robo stake (sum of all stakes)
  - Average price per RT
  - List of contract IDs
  - Deterministic ingot ID (timestamp-based)
- Maintains carryover for excess joules

**Threshold Logic**:
```go
accumulatedJoules += ore.Joules
if accumulatedJoules >= 3600 {
    // Create ingot with exactly 3600J
    ingot := createIngot(3600, contracts, roboStake, avgPrice)
    // Carry over excess
    carryoverJoules = accumulatedJoules - 3600
    accumulatedJoules = carryoverJoules
}
```

**Hash Generation**:
- Combines contract IDs, joules, robo stake, timestamp
- Uses SHA-256 for deterministic ingot identification

### 4. BatchSender (`internal/refinery/batch_sender.go`)

**Purpose**: Periodically sends completed ingots to Mint

**Responsibilities**:
- Runs as a goroutine with interval-based timer
- Collects completed ingots from IngotAssembler
- Publishes batches via MintClient

**Important Behavior**:
- Only sends **completed ingots** (already assembled at 3600J)
- Does NOT force assembly of partial ingots at interval
- Skips batch send if no completed ingots available
- On shutdown, sends any remaining ingots before exiting

**Configuration**:
- `REFINERY_INGOT_BATCH_INTERVAL` (default: 60 seconds)

### 5. MintClient (`internal/refinery/mint_client.go`)

**Purpose**: NATS messaging client for publishing to Mint

**Responsibilities**:
- Establishes and maintains NATS connection
- Publishes batch envelopes to `mint.ingots` topic
- Implements retry logic with exponential backoff
- Handles automatic reconnection

**Batch Envelope Structure**:
```json
{
  "batch_id": "UUID",
  "timestamp": "2025-11-14T22:17:33Z",
  "ingot_count": 1,
  "ingots": [
    {
      "ingot_id": "20251114-221714.361906",
      "joule_torq_total": 3600,
      "robostake_torq_total": 0.01664,
      "price_per_rt": 14423.08,
      "contract_ids": ["contract-1", "contract-2", ...],
      "ore_hashes": ["hash1", "hash2", ...]
    }
  ]
}
```

**Retry Configuration**:
- `MINT_MAX_RETRIES` (default: 3)
- `MINT_BASE_DELAY` (default: 1 second)
- Exponential backoff: 1s, 2s, 4s, ...

**Resilience**:
- NATS Go client buffers messages during disconnection
- Automatic reconnection with internal backoff
- Connection status exposed via health endpoint

### 6. Config (`internal/config/config.go`)

**Purpose**: Centralized configuration management

**Environment Variables**:
```bash
NATS_URL=nats://nats:4222
REFINERY_INGOT_BATCH_INTERVAL=60  # seconds
MINT_MAX_RETRIES=3
MINT_BASE_DELAY=1  # seconds
REFINERY_JOULE_QUEUE_SIZE=1000
REFINERY_ROBO_QUEUE_SIZE=1000
```

**Defaults**: All values have sensible defaults for development

### 7. Metrics (`internal/metrics/metrics.go`)

**Prometheus Metrics**:
- `refinery_ore_received_total` - Total ore received
- `refinery_ore_validation_errors_total` - Validation failures
- `refinery_queue_full_total` - Backpressure events
- `refinery_ingots_assembled_total` - Total ingots created
- `refinery_batches_sent_total` - Total batches published
- `refinery_ingots_sent_total` - Total ingots published
- `refinery_batch_send_duration_seconds` - Batch publish latency
- `refinery_batch_size` - Ingots per batch distribution

**Endpoints**:
- `GET /metrics` - Prometheus scrape endpoint
- `GET /health` - Health check with queue and NATS status

### 8. Health Handler (`internal/refinery/health_handler.go`)

**Purpose**: Service health monitoring

**Health Response**:
```json
{
  "status": "healthy",  // or "degraded"
  "timestamp": "2025-11-14T22:17:33Z",
  "queues": {
    "joule_queue_size": 0,
    "joule_queue_capacity": 1000,
    "joule_queue_usage_percent": 0,
    "robo_queue_size": 0,
    "robo_queue_capacity": 1000,
    "robo_queue_usage_percent": 0
  },
  "nats": {
    "connected": true,
    "status": "CONNECTED"  // or "RECONNECTING"
  },
  "ingot_assembly": {
    "accumulated_joules": 900,
    "completed_ingots_pending": 0,
    "progress_to_next_ingot_percent": 25
  }
}
```

**Status Determination**:
- `healthy`: NATS connected, queues < 90% full
- `degraded`: NATS disconnected or queues > 90% full

### 9. Graceful Shutdown

**Implementation**:
- Uses `context.WithCancel()` for coordination
- Signal handling: SIGINT, SIGTERM
- Shutdown sequence:
  1. Stop accepting new HTTP requests
  2. Cancel context (signals all goroutines)
  3. BatchSender sends remaining ingots
  4. IngotAssembler stops processing
  5. HTTP server graceful shutdown (5s timeout)
  6. QueueManager reports remaining items
  7. MintClient closes NATS connection

**Verification**:
```bash
docker-compose stop refinery
docker logs robotorq-network-refinery-1 --tail 10
# Should show: "shutdown signal received", "batch sender shutting down", etc.
```

## Data Flow

### Ore Reception → Ingot Assembly → Batch Publishing

1. **Digger** sends `POST /receive-ore` with `JouleTorqOre`
2. **OreReceiver** validates and creates queue items
3. **QueueManager** buffers in channels (non-blocking)
4. **IngotAssembler** (goroutine) consumes from both queues:
   - Accumulates joules
   - Tracks contracts, robo stake, prices
   - At 3600J threshold: creates `TokenTorqIngot`
5. **BatchSender** (goroutine) timer fires every 60s:
   - Collects completed ingots
   - Calls MintClient.PublishBatch()
6. **MintClient** publishes to NATS `mint.ingots`
7. **Mint** service receives and processes

### Carryover Example

```
Ore 1: 900J  → Accumulated: 900J (no ingot)
Ore 2: 900J  → Accumulated: 1800J (no ingot)
Ore 3: 900J  → Accumulated: 2700J (no ingot)
Ore 4: 900J  → Accumulated: 3600J → **Ingot Created** (3600J) → Carryover: 0J
Ore 5: 1000J → Accumulated: 1000J (no ingot, starts next cycle)
```

### Multiple Contracts in Ingot

An ingot can contain ore from multiple contracts:
```
Ore 1: contract-A, 900J  → Accumulated: 900J
Ore 2: contract-B, 900J  → Accumulated: 1800J
Ore 3: contract-C, 900J  → Accumulated: 2700J
Ore 4: contract-A, 900J  → Accumulated: 3600J → Ingot with [contract-A, contract-B, contract-C]
```

## Deployment

### Docker Multi-Stage Build

**Dockerfile**:
- Stage 1 (builder): Compiles Go binary with `CGO_ENABLED=0`
- Stage 2 (runtime): Minimal Alpine image with binary only
- Final image: ~15MB (vs ~300MB+ with build tools)

**Build**:
```bash
docker build -t refinery:latest .
```

### Docker Compose

**Dependencies**:
- NATS (service_healthy condition)
- PostgreSQL (service_healthy condition)

**Ports**:
- 8080: HTTP server (internal)
- 8081: HTTP server (host mapping)
- 50052: gRPC (unused, legacy)

**Health Check**:
```bash
wget -qO- http://localhost:8080/health || exit 1
```

**Start**:
```bash
docker-compose up -d refinery
```

## Testing

### Test Coverage

- **Unit Tests**: 49 tests (ore receiver, queue manager, ingot assembler, mint client)
- **Integration Tests**: 4 scenarios (end-to-end, multiple ingots, HTTP validation, backpressure)
- **Total**: 53 tests + 4 benchmarks (100% passing)

### Manual Testing Scenarios

1. **Basic Ore Reception**: Send single ore, verify acceptance
2. **Threshold Triggering**: Send 4×900J ores, verify ingot at 3600J
3. **Batch Interval**: Verify 60s interval sends completed ingots
4. **NATS Connectivity**: Stop NATS, verify reconnection and buffering
5. **Graceful Shutdown**: Send SIGTERM, verify clean exit

See `TESTING_PLAN.md` for detailed test procedures.

## Monitoring

### Prometheus Metrics

**Scrape Configuration** (`prometheus.yml`):
```yaml
scrape_configs:
  - job_name: 'refinery'
    static_configs:
      - targets: ['refinery:8080']
```

### Grafana Dashboard

Key metrics to monitor:
- Ore reception rate (`refinery_ore_received_total`)
- Ingot assembly rate (`refinery_ingots_assembled_total`)
- Queue usage (`refinery_queue_usage_percent`)
- NATS connection status
- Batch publish latency (`refinery_batch_send_duration_seconds`)

## Configuration Tuning

### High Throughput

If receiving > 1000 ores/minute:
```bash
REFINERY_JOULE_QUEUE_SIZE=5000
REFINERY_ROBO_QUEUE_SIZE=5000
REFINERY_INGOT_BATCH_INTERVAL=30  # Send batches more frequently
```

### Low Latency

For faster batch publishing:
```bash
REFINERY_INGOT_BATCH_INTERVAL=10  # Send every 10 seconds
```

### NATS Resilience

For unstable networks:
```bash
MINT_MAX_RETRIES=10
MINT_BASE_DELAY=2  # Longer initial delay
```

## Security Considerations

1. **Input Validation**: All ore fields validated before processing
2. **Hash Verification**: SHA-256 hashes for ore integrity (future: signatures)
3. **TLS**: NATS supports TLS (configure with nats://... URLs)
4. **Rate Limiting**: Queue backpressure prevents memory exhaustion
5. **No Secrets in Logs**: Structured logging avoids leaking sensitive data

## Future Enhancements

1. **Proof of Work Validation**: Verify `proof_of_work` field from Digger
2. **Cryptographic Signatures**: Validate Dilithium signatures on ore
3. **Dead Letter Queue**: For failed NATS publishes after retries
4. **Persistent State**: Save accumulated joules to disk for crash recovery
5. **Dynamic Threshold**: Adjust ingot size based on network conditions
6. **Horizontal Scaling**: Multiple refinery instances with NATS queue groups

## Troubleshooting

### Queues Full (HTTP 429)

**Symptom**: Diggers receive "queue full" errors

**Solutions**:
1. Increase queue sizes
2. Check IngotAssembler goroutine is running
3. Verify NATS connection (queues drain when batches send)

### NATS Disconnected

**Symptom**: Health status "degraded", NATS "RECONNECTING"

**Solutions**:
1. Check NATS container: `docker ps | grep nats`
2. Check network: `docker network inspect torqnet`
3. Review NATS logs: `docker logs robotorq-network-nats-1`

### Ingots Not Assembling

**Symptom**: Accumulated joules stuck below 3600J

**Solutions**:
1. Send more ore to reach threshold
2. Check for validation errors in logs
3. Verify IngotAssembler goroutine started (log: "starting ingot assembler")

### Graceful Shutdown Timeout

**Symptom**: Container takes > 10s to stop

**Solutions**:
1. Reduce batch interval (less time waiting for timer)
2. Check for stuck goroutines (enable pprof)
3. Review shutdown logs for blocking operations

## References

- [Go Context Package](https://pkg.go.dev/context)
- [NATS Go Client](https://github.com/nats-io/nats.go)
- [Prometheus Go Client](https://github.com/prometheus/client_golang)
- [RoboTorq Whitepaper](../../docs/trust-economics.md)
