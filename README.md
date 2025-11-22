# RoboTorq Reserve System

Physics‑based monetary system backed by verified robotic labor.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?logo=docker)](https://www.docker.com/)

---

## TL;DR
- Not a cryptocurrency: no mining, no gas, no chain bloat.
- 1 RoboTorq = 1 hour of cryptographically proven robotic labor.
- Proof chain compresses raw token work → ingots → certificates.
- Demurrage (hoarding fee) drives circulation + funds operations.
- One command to run: `docker compose up -d`.
- Everything merklized & signed at each layer.

---

## Unit Hierarchy
| Layer | Artifact | Aggregation | Resulting Count | Crypto Operation | Role |
|-------|----------|-------------|-----------------|------------------|------|
| L0 | JouleTorqUnit | 1 token × measured joules | 3,600 per ingot | Hash + Sign | Atomic work proof |
| L1 | TokenTorqIngot | 3,600 units | 1,000 per certificate | Merkle branch root + Sign | Lossless compression |
| L2 | RoboTorq Certificate | 1,000 ingots (3.6M units) | Basis for reserve | Merkle batch root + Sign | Monetary issuance |
| L3 | Distribution Record | Certificate schedule | Dynamic | Time lock + Demurrage | Circulation enforcement |

Formula relationships:
`1 JouleTorqUnit → 3,600 = 1 TokenTorqIngot`
`1 TokenTorqIngot × 1,000 = 1 RoboTorq Certificate (3.6M units)`

Extended economic flow & vault mechanics: see [`docs/ECONOMICS_OVERVIEW.md`](./docs/ECONOMICS_OVERVIEW.md).

---

## Architecture (High Level)
```
Work (Digger) → Aggregation (Refinery) → Minting (Mint) → Distribution (DistoDam + Vault) → Wallets
   |                |                         |                    |
  Units          Ingots                  Certificates        Scheduled Streams
```
Key services: Digger (work measurement) • Refinery (ingot build) • Mint (validation + certificate merkle) • DistoDam (to be deprecated for pure vaulting, but does payout) • Wallet (balances) • Printer (registration) • Trust (future) • Vault (reserve logic).
Detailed conceptual intro: [`MENTAL_MODEL.md`](./MENTAL_MODEL.md).

---

## Quick Start
```bash
git clone https://github.com/GrokkingGrok/robotorq-reserve-system.git
cd robotorq-reserve-system
docker compose up -d
docker compose ps            # all services “Up”
```
Logs (example): `docker compose logs -f mint`.
Monitoring: Prometheus http://localhost:9090 • Grafana http://localhost:3000 (admin/admin).

Stop:
```bash
docker compose down
docker compose down -v   # full reset
```

---

## Features
- Physics‑anchored value (energy + tokens/sec).
- Multi‑layer merkle compression (3.6M unit proofs → one certificate root).
- Demurrage distribution (UBD) incentivizes active circulation.
- All services containerized; zero local toolchain setup.
- Structured Git workflow (see `BRANCHING.md`).
- Observability baked in (Prometheus + curated Grafana dashboards).

---

## Ports (Grouped)
| Category | Ports | Description |
|----------|-------|-------------|
| Core Messaging | 4222 / 8222 | NATS broker / monitor |
| Work & Proof APIs | 3030 / 8081 / 8080 | Digger / Refinery / Mint HTTP |
| Distribution & Accounts | 8082 / 8085 / 8083 | DistoDam / Wallet / Trust |
| Printer | 9092 / 9094 | Mock API / Metrics |
| Observability | 9090 / 9091 / 3000 | Mint metrics / Prometheus / Grafana |
| gRPC | 50051 / 50052 | Mint / Refinery gRPC |
| Database (internal) | 5432 | Postgres |
Full table: [`port mapping/PORT_MAPPINGS.md`](./port%20mapping/PORT_MAPPINGS.md).

---

## Testing
Go unit tests:
```bash
go test ./... -v -race -cover
```
Python integration/e2e:
```bash
pytest tests/integration -v
python tests/e2e/phase5_verification_flow.py
```
More: [`tests/README.md`](./tests/README.md).

---

## Contributing (Essentials)
- Branch from `main`: `git checkout -b feature/<name>`.
- Conventional Commits (`feat:`, `fix:`, `docs:`...).
- Add / update tests with changes (target ≥80% unit + integration coverage for new code).
- Keep architecture docs in sync (Mint / Refinery / Vault).
Full workflow: [`BRANCHING.md`](./BRANCHING.md).

---

## Deep Dives
- Proof Chain: [`docs/PROOF_CHAIN_ARCHITECTURE.md`](./docs/PROOF_CHAIN_ARCHITECTURE.md)
- Mint Design: [`src/mint/MINT_ARCHITECTURE.md`](./src/mint/MINT_ARCHITECTURE.md)
- Refinery Design: [`src/refinery/REFINERY_ARCHITECTURE.md`](./src/refinery/REFINERY_ARCHITECTURE.md)
- Vault Final Design: [`src/vault/Architecture/Final design/VAULT_MVP_FINAL_DESIGN.md`](./src/vault/Architecture/Final%20design/VAULT_MVP_FINAL_DESIGN.md)
- Economics Overview: [`docs/ECONOMICS_OVERVIEW.md`](./docs/ECONOMICS_OVERVIEW.md)
- Trust Economics: [`docs/trust-economics.md`](./docs/trust-economics.md)

---

## Security
Apache 2.0 + Patent Pledge (see `LICENSE`).
Every layer signed; merkle roots provide integrity. Formal SECURITY.md pending pre‑release.

---
## Philosophy
“Watts > Wall Street” – Monetary value should reflect real, measured robotic labor, not speculative scarcity. RoboTorq encodes energy → cryptographic proof → circulating value with built‑in incentives to keep it moving.

---

## Status
Phase: v0 cleanup (branch `release/v0-cleanup`).
Working: full proof chain, merkle compression, demurrage distribution, dashboards.
In Progress: trust service, deeper security audit, production hardening.

---

## License & Patent Pledge
Apache License 2.0. All contributors irrevocably dedicate any patent rights and pledge non‑assertion against users and derivatives. See [`LICENSE`](./LICENSE).

---

## Contact
Issues / feedback: https://github.com/GrokkingGrok/robotorq-reserve-system/issues

Built with ⚡ by robots, for robots (and humans who benefit).

---

## Architecture Overview

RoboTorq uses a **message-passing architecture** (NATS pub/sub) with services that aggregate proofs:

```
Digger (Rust)     →  Refinery (Go)   →  Mint (Go)        →  DistoDam (Go)
[Does Work]          [Aggregates]       [Validates]          [Distributes]
  ↓                      ↓                  ↓                    ↓
JouleTorqUnits    TokenTorqIngots    RoboTorqBatches      Verified Units
(individual)      (3600 units)       (1000 ingots)        (to wallets)
```

**Key Services**:
- **Digger**: Executes AI inference contracts, measures energy, creates signed proof units
- **Refinery**: Aggregates 3,600 proof units into ingots with merkle branch hashes
- **Mint**: Validates 1,000 ingots, builds merkle tree, mints 1 RoboTorq unit
- **DistoDam**: Distributes minted units to wallets based on demurrage model
- **Wallet**: Manages balances, transactions, and unit verification
- **Printer**: Registration service for new diggers joining the network

**Architecture Details**: See [`MENTAL_MODEL.md`](./MENTAL_MODEL.md) for complete proof chain walkthrough.

---

## Prerequisites

- **Docker** (with Docker Compose V2)
- **Git**

That's it. Everything runs in containers.

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/GrokkingGrok/robotorq-reserve-system.git
cd robotorq-reserve-system
```

### 2. Start All Services

```bash
docker compose up -d
```

This will:
- Build all service images (first run takes ~5-10 minutes)
- Start NATS message broker, PostgreSQL, Prometheus, Grafana
- Launch all RoboTorq services (Refinery, Mint, DistoDam, Trust, Printer, Wallet)

### 3. Verify Services Are Running

```bash
docker compose ps
```

Expected output: All services should show `Up` status with healthy checks passing.

### 4. Check Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f mint
docker compose logs -f refinery
```

### 5. Access Monitoring (Optional)

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000
  - Default credentials: `admin` / `admin`
  - Pre-configured dashboard: "RoboTorq Complete Pipeline"

---

## Running a Test Flow

To simulate a complete work → mint → distribution cycle, use the test scripts:

```bash
# Prerequisites: Python 3.10+ with nats-py
pip install nats-py

# Activate a wallet for receiving units
python scripts/activate_wallet.py

# Enable distribution (allows DistoDam to send units)
python scripts/enable_wallet_distribution.py

# Monitor the full pipeline
python scripts/test_full_pipeline.py
```

**Note**: For headless digger execution (actual work simulation), see [`src/digger/README.md`](./src/digger/README.md).

---

## Project Structure

```
robotorq-reserve-system/
├── src/
│   ├── digger/         # Rust: AI inference + energy measurement
│   ├── refinery/       # Go: Unit aggregation into ingots
│   ├── mint/           # Go: Ingot validation + RoboTorq minting
│   ├── distodam/       # Go: Distribution with demurrage
│   ├── wallet/         # Go: Balance + transaction management
│   ├── printer/        # Go: Digger registration service
│   ├── trust/          # Go: Reputation + fraud detection (future)
│   └── vault/          # Architecture docs for reserve system (planned)
├── tests/
│   ├── e2e/            # End-to-end pipeline tests
│   ├── integration/    # Service integration tests
│   └── fixtures/       # Shared test helpers
├── scripts/            # Python utilities for testing/monitoring
├── docs/               # Architecture and phase completion docs
├── Grafana/            # Monitoring dashboards
├── docker-compose.yaml # Complete service orchestration
├── MENTAL_MODEL.md     # Conceptual architecture guide
├── BRANCHING.md        # Git workflow and development process
└── LICENSE             # Apache 2.0 with patent pledge
```

---

## Key Concepts

### Energy-Based Value
Unlike cryptocurrencies that waste energy on proof-of-work, RoboTorq measures **productive energy consumption**:
- Real computational work (AI inference, data processing)
- Measured in joules (watts × seconds)
- Cryptographically signed at every stage

### Proof Chain Integrity
Every RoboTorq unit contains a complete proof chain:
1. **Layer 0**: Individual token hashes (JouleTorqUnit)
2. **Layer 1**: Aggregated merkle branch hash (TokenTorqIngot - 3,600 units)
3. **Layer 2**: Batch merkle root (RoboTorqBatch - 1,000 ingots)
4. **Layer 3**: Final unit with complete verification path

### Demurrage Model
- Units carry a 0.5% monthly holding fee (extracted by DistoDam)
- Encourages circulation over hoarding
- Fees fund system operations and infrastructure

### No Installation Required
Services communicate via NATS; no local Python/Go/Rust environments needed. Docker handles all dependencies, builds, and networking.

---

## Development Workflow

See [`BRANCHING.md`](./BRANCHING.md) for the complete development workflow, including:
- Branch strategy (feature branches from `main`)
- Commit conventions (Conventional Commits)
- Testing requirements (95%+ coverage target)
- The proven 14-step Power Workflow

**Quick Summary**:
1. Create feature branch: `git checkout -b feature/your-feature`
2. Read relevant architecture docs in `src/{service}/`
3. Implement with tests (Go: `go test ./...`, Python: `pytest`)
4. Commit incrementally with descriptive messages
5. Open PR to `main` when ready

---

## Port Reference

See [`port mapping/PORT_MAPPINGS.md`](./port%20mapping/PORT_MAPPINGS.md) for complete port allocations to avoid conflicts.

**Key Ports**:
- `3000`: Grafana dashboard
- `3030`: Digger HTTP API
- `4222`: NATS messaging
- `5432`: PostgreSQL (internal only)
- `8080`: Mint HTTP API
- `8081`: Refinery HTTP API
- `8082`: DistoDam HTTP API
- `8083`: Trust HTTP API
- `8085`: Wallet HTTP API
- `9090`: Mint Prometheus metrics
- `9091`: Prometheus UI
- `9092`: Printer mock API
- `9094`: Printer metrics

---

## Testing

### Unit Tests (Go)
```bash
# Run all Go tests with coverage
cd src/mint  # or refinery, distodam, etc.
go test ./... -v -cover -race
```

### Integration Tests (Python)
```bash
cd tests
pytest integration/ -v
```

### End-to-End Tests (Python)
```bash
cd tests
python e2e/phase5_verification_flow.py
```

**Test Documentation**: See [`tests/README.md`](./tests/README.md)

---

## Stopping Services

```bash
# Stop all services
docker compose down

# Stop and remove volumes (clean slate)
docker compose down -v
```

---

## Architecture Deep Dives

- **Mental Model**: [`MENTAL_MODEL.md`](./MENTAL_MODEL.md) - Start here for conceptual understanding
- **Proof Chain**: [`docs/PROOF_CHAIN_ARCHITECTURE.md`](./docs/PROOF_CHAIN_ARCHITECTURE.md)
- **Mint**: [`src/mint/MINT_ARCHITECTURE.md`](./src/mint/MINT_ARCHITECTURE.md)
- **Refinery**: [`src/refinery/REFINERY_ARCHITECTURE.md`](./src/refinery/REFINERY_ARCHITECTURE.md)
- **Vault Design**: [`src/vault/Architecture/Final design/VAULT_MVP_FINAL_DESIGN.md`](./src/vault/Architecture/Final%20design/VAULT_MVP_FINAL_DESIGN.md)
- **Trust Economics**: [`docs/trust-economics.md`](./docs/trust-economics.md)

---

## Contributing

Contributions welcome! Please see [`CONTRIBUTING.md`](./CONTRIBUTING.md) (coming soon) for guidelines.

**Key Points**:
- Follow the Power Workflow in `BRANCHING.md`
- Write tests (80%+ unit coverage target, cover remaining with integration and e2e)
- Use Conventional Commits format
- Update architecture docs when changing designs

---

## Security

For security concerns or vulnerability reports, see [`SECURITY.md`](./SECURITY.md) (coming soon).

**Current Status**: Pre-release development on `release/v0-cleanup` branch. Not production-ready.

---

## License

Apache License 2.0 with Patent Pledge - see [`LICENSE`](./LICENSE) for details.

**Patent Pledge**: All contributors irrevocably dedicate patent rights to the public domain and pledge not to assert patent claims against users or derivatives.

---

## Philosophy

> **"Watts > Wall Street"**

RoboTorq challenges the assumption that money must be:
- Created by governments (fiat)
- Mined wastefully (Bitcoin)
- Backed by speculation (most altcoins)

Instead, we anchor value to **measurable physics**: energy and computation consumed performing useful robotic labor, where the selling price is arranged in advance, and therefore the value to generated as money and distributed is mathematically knowable. 

Every RoboTorq represents real work that actually happened, cryptographically proven at every layer.

This is not "blockchain for blockchain's sake"—it's a **reserve system** where robotic labor creates tangible value that can be verified, traded, and used to compensate humans in the form of a universal basic dividend

---

## Status

**Current Phase**: v0 Cleanup & Open Source Preparation  
**Branch**: `release/v0-cleanup`  
**Target**: Public release with complete documentation and security hardening

**What Works**:
- ✅ Complete proof chain (Digger → Refinery → Mint → DistoDam)
- ✅ Energy measurement and cryptographic signing
- ✅ Merkle tree aggregation at all layers
- ✅ Wallet management and transaction validation
- ✅ Demurrage-based distribution model
- ✅ Prometheus + Grafana observability

**In Progress**:
- 🚧 Trust service (reputation/fraud detection)
- 🚧 Comprehensive test coverage documentation
- 🚧 Security audit and hardening
- 🚧 Production deployment guides

---

## Contact

- **Repository**: https://github.com/GrokkingGrok/robotorq-reserve-system
- **Issues**: https://github.com/GrokkingGrok/robotorq-reserve-system/issues

---

**Built with ⚡ by robots, for robots (and the humans who benefit from their work)**
