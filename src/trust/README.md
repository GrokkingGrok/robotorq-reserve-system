# rust-trust

Minimal scaffold for the Rust `trust` service (MVP).

This crate serves the Genesis contract and replies to contract requests from Diggers over NATS.

See `RUST_TRUST_MVP_PLAN.md` and `RUST_TRUST_CONFIG.md` in the repo for design and configuration.
# Trust Service

The Trust service is a decentralized contract orchestration system that evaluates opportunities, creates contracts, and executes them through the Digger network.

## Architecture

### 5-Step Pipeline

```
1. Opportunity Intake
   ↓ (Ticker auto-generates OR HTTP POST /opportunities)
   
2. Appraisal
   ↓ (ROI >= 10% threshold)
   ↓ (Creates Contract)
   ↓ (Publishes to NATS: contracts.pending)
   
3. Funding (DistoDam)
   ↓ (DistoDam subscribes to contracts.pending)
   ↓ (Checks reservoir, funds contract)
   ↓ (Publishes to NATS: contracts.funded)
   
4. Fund Sync
   ↓ (FundSync subscribes to contracts.funded)
   ↓ (Forwards to Executor)
   
5. Execution
   ↓ (Executor calls Digger HTTP API)
   ↓ (GET /robot/status → POST /stake)
   ↓ (Contract status: "executing")
```

## Components

### Internal Packages

- **`appraiser/`** - Evaluates opportunities based on ROI threshold (10%)
- **`contract/`** - Contract data model with lifecycle timestamps
- **`executor/`** - HTTP client for Digger API integration
- **`fundsync/`** - NATS subscriber bridging funding events to execution
- **`httpapi/`** - REST endpoints (`/opportunities`, `/metrics`, `/health`)
- **`metrics/`** - Thread-safe atomic counters for observability
- **`opportunity/`** - Opportunity data model with thread-safe mutations
- **`ticker/`** - Auto-generates test opportunities (development/testing)
- **`trustsvc/`** - Service orchestration and wiring

### Key Features

- **ROI-Based Appraisal**: Automatically approves opportunities with ROI ≥ 10%
- **Decentralized Funding**: DistoDam independently decides to fund contracts
- **Asynchronous Execution**: NATS messaging decouples components
- **Worker Pools**: Concurrent processing with configurable parallelism
- **Prometheus Metrics**: 7 tracked metrics for full pipeline visibility

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NATS_URL` | `nats://localhost:4222` | NATS server connection string |
| `PORT` | `8080` | HTTP server port |

### NATS Topics

- **`contracts.pending`** (Published): Contracts awaiting funding from DistoDam
- **`contracts.funded`** (Subscribed): Contracts funded by DistoDam, ready for execution

## API Endpoints

### `POST /opportunities`
Submit opportunities for appraisal.

**Request Body**:
```json
[
  {
    "id": "opp-001",
    "builder": "alice",
    "description": "Build AI model",
    "digger_url": "http://localhost:9000",
    "required_rt": 1000,
    "roi": 0.15
  }
]
```

**Response**: `202 Accepted`

### `GET /metrics`
Retrieve pipeline metrics.

**Response**:
```json
{
  "submitted": 123,
  "appraised": 123,
  "contracts_created": 98,
  "funds_synced": 45,
  "contracts_funded": 45,
  "executions": 42,
  "contracts_executed": 42
}
```

### `GET /health`
Health check endpoint.

**Response**: `200 OK` with body `"ok"`

## Development

### Running Locally

```bash
# Start NATS
docker compose up -d nats

# Set environment
export NATS_URL="nats://localhost:4222"

# Run Trust service
cd src/trust
go run ./cmd/trust
```

### Running with Docker Compose

```bash
# Build and start all services
docker compose up -d trust

# View logs
docker compose logs -f trust

# Check metrics
curl http://localhost:8083/metrics
```

### Testing

```bash
# Run tests
cd src/trust
go test ./...

# Send test opportunity
curl -X POST http://localhost:8083/opportunities \
  -H "Content-Type: application/json" \
  -d '[{
    "id": "test-001",
    "builder": "test-builder",
    "description": "Test opportunity",
    "digger_url": "http://localhost:9000",
    "required_rt": 500,
    "roi": 0.12
  }]'
```

## Metrics

The service exposes 7 key metrics via `/metrics`:

1. **`submitted`** - Opportunities received via HTTP or Ticker
2. **`appraised`** - Opportunities evaluated (approved or rejected)
3. **`contracts_created`** - Contracts created from approved opportunities
4. **`contracts_funded`** - Contracts funded by DistoDam
5. **`funds_synced`** - Funded contracts forwarded to Executor
6. **`executions`** - Execution attempts sent to Digger
7. **`contracts_executed`** - Successfully executed contracts

## Integration with Ecosystem

### Dependencies
- **NATS**: Message broker for pub/sub coordination
- **DistoDam**: Funding reservoir (subscribes to `contracts.pending`)
- **Digger**: HTTP API for contract execution (`/robot/status`, `/stake`)

### Data Flow
1. Trust receives opportunities → creates contracts → publishes `contracts.pending`
2. DistoDam funds contracts → publishes `contracts.funded`
3. Trust FundSync receives `contracts.funded` → Executor calls Digger API
4. Digger processes work → produces tokens

## Port Configuration

- **Development**: Port `8080` (local)
- **Docker**: Internal `8080`, exposed as `8083` (to avoid conflicts)
- **CI/CD**: Port `8080` (containerized environment)

## Future Enhancements

- Retry logic and dead-letter queue for failed executions
- Dynamic ROI threshold configuration
- Advanced appraisal algorithms (ML-based scoring)
- Multi-region NATS cluster support
- Contract cancellation and refund flows
