# The RoboTorq Reserve System

## What is RoboTorq?

A deterministic, physics-backed reserve currency generated from cryptographically verified robotic labor, without blockchain mining or speculative inflation. Minted digitally, it can be redeemed physically from any 3D printer.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?logo=docker)](https://www.docker.com/)
![Status](https://img.shields.io/badge/status-pre--release-orange)

---

## TL;DR
- Not a cryptocurrency: no mining, no gas, no chain bloat.
- 1 RoboTorq = 1 hour of cryptographically proven robotic labor.
- Proof chain compresses raw token work → ingots → certificates.
- Demurrage (hoarding fee) drives circulation + funds operations.
- One command to run: `docker compose up -d`.
- Everything merklized & signed at each layer.

---

## TOC

- [Philosophy](#philosophy)
- [Why No Blockchain?](#why-no-blockchain)
- [Unit Hierarchy](#unit-hierarchy)
- [Quick Start](#quick-start)
- [Testing](#testing)
- [Architecture Overview](#architecture-overview)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Architecture Deep Dives](#architecture-deep-dives)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)
- [Status](#status)
- [Contact](#contact)

---

## Philosophy

> **"Watts > Wall Street"**

The RoboTorq Reserve System challenges the assumption that money must be:
- Created by governments (centralized)
- Mined wastefully (Bitcoin)
- Backed by speculation (fiat/altcoins)

Instead, we anchor value to **measurable physics**: energy and computation consumed performing useful robotic labor, where the selling price is arranged in advance, and therefore the value to be generated as money and distributed is mathematically knowable. 

Every RoboTorq represents real work that actually happened, cryptographically proven at every layer.

This is not "blockchain for blockchain's sake"—it's a **reserve system** where robotic labor creates tangible value that can be verified, traded, and used to compensate humans in the form of a universal basic dividend.

## Why No Blockchain?

RoboTorq uses a lightweight message-passing network (NATS) and certificate-backing instead of blockchain consensus.

All proofs are signed, merklized, and linked, but RoboTorq Certificates never leave the distributed vaults.

There is **no mining**, **no global transaction ledger**, and **no chain data bloat**.

The RoboTorq Reserve System is now and always will be lightweight enough to run on about a dozen Raspberry Pis — by design, not as a stunt. The architecture scales horizontally, making it easily deployable even on low-power hardware. Citizens of developing countries can operate a full collection of nodes just as effectively as those in developed nations.


---

## Unit Hierarchy
| Layer | Artifact | Aggregation | Resulting Count | Crypto Operation | Role |
|-------|----------|-------------|-----------------|------------------|------|
| L0 | JouleTorqOre | 1 token × 1 joule | 3,600 per ingot | Raw Data Hashed + Signed | Atomic work proof |
| L1 | TokenTorqIngot | 3,600 units of Ore | 1,000 per certificate | Merkle branch root + Signed | Batched for minting |
| L2 | RoboTorq Certificate | 1,000 ingots (3.6M units of Ore) | Basis for reserve | Merkle batch root + Signed | Monetary Backing |
| L3 | RoboTorqUnits | Certificate-Backed, Digital, 1:1 | Dynamic | Signed Distribution Events | Circulation |
| L4 | Bearer Bond | Certificate-Backed, Physcal, 1:1 | Dynamic | Merkle Cert Collection + Signed | Circulation |

Formula relationships:
`1 TokenTorqIngot = 3,600 JouleTorqOre units`
`1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units`

Extended economic flow & vault mechanics: see [`docs/ECONOMICS_OVERVIEW.md`](./docs/ECONOMICS_OVERVIEW.md).


---

## Quick Start

```bash
git clone https://github.com/GrokkingGrok/robotorq-reserve-system.git
cd robotorq-reserve-system
docker compose up -d
docker compose ps            # all services “Up”
```

**Logs**: 
```bash
# docker native log reader, follow stream
docker compose logs -f mint 

# check scripts/logs folder for helpful python log checkers if you don't want to learn docker logging

# View last 20 lines, no follow
python scripts/logs/mintlogs.py 
# View last N lines only, no follow
python scripts/logs/mintlogs.py --tail 100
# Follow logs in real-time (skip initial tail, stream continuously)
python scripts/logs/mintlogs.py -f
```

**Monitoring**: 
- Grafana http://localhost:3000 (admin/admin)
- Grafana/robotorq-complete-pipeline.json (fully configured pipeline viewer definition)


**Stop**:
```bash
docker compose down
docker compose down -v   # full reset
```

---

## Prerequisites

- **Git** to clone repo
- **Docker** (with Docker Compose V2) to run nodes
- **Python** for testing

That's it. Everything runs in containers.

## Testing

### Unit Tests (Go)
```bash
# Run all Go tests with coverage
cd src/mint  # or refinery, distodam, etc.
go test ./... -v -cover -race
```

---

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

## Architecture Overview

RoboTorq uses a **message-passing architecture** (NATS pub/sub) with services that aggregate proofs:

**Key Services**:
- **Digger**: Tracks contracted robotic labor, measures energy, creates signed proof units of Ore
- **Refinery**: Aggregates 3,600 proof units into ingots with merkle branch hashes
- **Mint**: Validates 1,000 ingots, builds merkle tree, mints 1 RoboTorq unit
- **DistoDam**: Distributes minted units to wallets based on demurrage model
- **Wallet**: Manages balances, transactions, and unit verification
- **Printer**: Registration service for new diggers joining the network

---

### High-Level Flow

`Robot → Digger → Refinery → Mint → DistoDam → Wallet`

---

## Project Structure

```
robotorq-reserve-system/
├── src/
│   ├── bidnet/         # Rust: (future) Bid on Robotic Labor Projects or exchange currency
│   ├── digger/         # Rust: Track Robotic Labor/Generates Ore
│   ├── refinery/       # Go: Ore aggregation into ingots
│   ├── mint/           # Go: Ingot validation + RoboTorq minting
│   ├── distodam/       # Go: Distribution with demurrage
│   ├── wallet/         # Go: Balance + transaction management
│   ├── printer/        # Go: A Mock 3D printer to act as robot's labor for tracking, will also print RoboTorq
│   ├── trust/          # Go: Contract Execution
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

## Status

**Current Phase**: v0 Cleanup & Open Source Preparation  
**Branch**: `release/v0`  
**Target**: Public release with working crypto pipeline, extensive documentation, and observability.

**What Works**:
- Complete proof chain (Robot → Digger → Refinery → Mint → DistoDam)
- Energy tracking and cryptographic signing
- Merkle tree aggregation at all layers (except physical bearer bond merkles, depends on vault)
- Wallet recieves UBD, but cannot spend
- Prometheus + Grafana observability (Watch the cypto pipeline in action)

**In Progress**:
- Vault System fully designed, but not implemented.
- Comprehensive documentation
- Security audit and hardening
- Trust service (Executes Contracts)
- BidNet (invest in robotic labor projects)

---

## Contact

- **Repository**: https://github.com/GrokkingGrok/robotorq-reserve-system
- **Issues**: https://github.com/GrokkingGrok/robotorq-reserve-system/issues

---

**Minted with ⚡ by bots, for humans**