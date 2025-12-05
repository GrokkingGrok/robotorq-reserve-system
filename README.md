# The RoboTorq Reserve System Paper
*Author*: Jonathan Clark

The RoboTorq Reserve System is an open source, decentralized, Universal Basic Dividend-paying monetary system.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?logo=docker)](https://www.docker.com/)
![Status](https://img.shields.io/badge/status-pre--release-orange)


## What is RoboTorq?

A deterministic, physics-backed reserve currency generated from cryptographically verified robotic labor, without blockchain mining or speculative inflation. Minted and traded digitally initially, the design also allows for RoboTorq Bearer Bonds to be redeemed physically from any 3D printer, at home, or stamped and sold *en masse*, all backed by on-grid, in-vault certificates on a distributed proof ledger.

---

## Executive TL;DR
- Not a "cryptocurrency": no blockchain → no mining → no chain bloat.
- 1 RoboTorq = Energy × Bonded Token Throughput (a cross-product)
- Specifically, every joule is mapped to a Bonded Token as each are consumed.
- Value emerges from both how much work is done and how efficiently it flows into the system.
- Proof chain compresses raw token x energy "ore" data → ingots → certificates.
- Demurrage (hoarding fee) drives circulation + funds new robotic labor to make new currency.
- One command to run: `docker compose up -d`.

## Abstract

The RoboTorq Reserve System circulates a certificate-backed currency in a closed-loop economy. The self-regulating system is designed such that the collateral can, mathematically speaking, never run dry... but only so long as people choose to keep using it.

How? Every atomic investment in robotic labor, the collectively paid `RoboStake`, serves as the value basis for minting more RoboTorq, bounded by the laws of physics and smart contracts. The closed loop design + contract approval vault check + demurrage ensures the distributed `StakeVault` never accepts a contract it can't fund.

The RoboTorq Reserve System isn't a government. It doesn't have to go into debt to pay a UBI. It simply accepts whatever growth stimulus we choose to give it, that it and we can already afford, and pays UBD based on that input.

Long-term financing is achieved with collateralized, vaulted savings tied to UBD pledges.

---

## TOC

- [Philosophy](#philosophy)
- [Why No Blockchain?](#why-no-blockchain)
- [Unit Hierarchy](#unit-hierarchy)
- [Persistence](#persistence)
- [Contact](#contact)

---

## Philosophy

*"Watts > Wall Street"*

RoboTorq challenges three assumptions:
- Money must be state-issued
- Mining must waste energy
- Value must be speculative

Instead, RoboTorq anchors value to *measurable physics* — actual energy and computation consumed by contracted robotic labor. Each unit is minted only after the work is performed, priced in advance, and cryptographically proven.

Every RoboTorq corresponds to real, verifiable work. This is not “blockchain because blockchain” (there is no blockchain). 

It’s a reserve system where robotic labor produces tangible, auditable, certificate-backed currency and enables universal basic dividends without political inflation games or Wall Street speculation.

Demurrage-free savings and investment schemas ensure liquidity, while demurrage on idle wallets yields interest to savers and funds Robotic Labor. All Bearer Bonds are demurrage exempt and can be printed at home because printing does not equal minting in the RoboTorq Reserve System.

## Why No Blockchain?

RoboTorq uses a lightweight, message-passing network (NATS), vaulted certificate-backed proofs, and cryptographically tracked issuance events instead of global consensus.
- Certificates remain inside distributed vaults
- All vaulted artifacts are signed, merklized, and linked
- No mining, no gas, no chain bloat, no global *transaction* ledger (proof ledgers are **tiny** by comparison)
- Track a large nation-state economy of yearly proofs on several terabytes of distributed storage.

The core system is intentionally compact — with each service designed to eventually run on a small cluster of Raspberry Pis + minimal storage if need be — so communities with limited resources could hypothetically operate local reserve system nodes on the same footing as anyone else.

Any productive machine (3D printers, CNCs, Cricuts, etc.) can act as a tracked robot, creating a low-barrier, closed-loop economy. If participants choose to use it, the system can sustain itself mathematically through continuous, provable robotic labor rather than speculative growth.

---

## Unit Hierarchy
| Layer | Artifact | Aggregation | Resulting Count | Crypto Operation | Role |
|-------|----------|-------------|-----------------|------------------|------|
| L0 | JouleTorqOre | 1 token × 1 joule | 3,600 per ingot | Raw Data Hashed + Signed | Atomic work proof |
| L1 | TokenTorqIngot | 3,600 units of Ore | 1,000 per certificate | Merkle branch root + Signed | Batched for minting |
| L2 | RoboTorq Certificate | 1,000 ingots (3.6M units of Ore) | Basis for reserve | Merkle batch root + Signed | Monetary Backing |
| L3 | RoboTorqUnits | Certificate-Backed, Digital, 1:1 | Dynamic | Signed Distribution Events | Circulation |
| L4 | Bearer Bond | Certificate-Backed, Physical, N:1 mapping | Dynamic | Merkle Cert Collection + Signed | Circulation |

Formula relationships:
`1 TokenTorqIngot = 3,600 JouleTorqOre units`
`1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units`

---

## Schema Versioning

Commons types that are serialized and hashed carry an explicit `schema_version` field or are covered by central version constants in `util/schema`.

- Central registry: see `util/schema/schema.rs` for per-type constants and helpers (`current_schema_version`, `all_schema_versions`).
- Included types: `Token`, `Robot`, `TripleTorq`, `UnmappedOreBatch`, runtime `RoboTorqConfig`, and all ID newtypes (constants only).
- Policy:
	- Increment on backward-incompatible changes to a type’s serialized shape.
	- Minor additions that are backward-compatible may leave the version unchanged; bump if consumers require it.
	- Migration/validation helpers can query `current_schema_version(type_name)` to compare stored vs current versions.
- Metrics: the `MetricsHandler` provides `register_schema_version_gauges(prefix)` to export a gauge per type (e.g., `commons_schema_version_token`).

---

## Contact

- **Repository**: https://github.com/GrokkingGrok/robotorq-reserve-system
- **Issues**: https://github.com/GrokkingGrok/robotorq-reserve-system/issues

---

**Minted with ⚡ by bots, for humans**

---

**AI DISCLOSURE & AUTHOR NOTE**: This document was written by and is maintained by Jonathan Clark (human). It was and will continue to be edited/refined by several AI agents as part of that efficient maintenance.

Other parts of the repository — including many architecture deep-dive docs, and yes, CODE — were drafted or expanded by AI in close collaboration with me, based on long chats. They serve as living design notebooks and current implementations: reference material for myself and future contributors to challenge, refine, or replace — NOT GOSPEL. Final responsibility for everything (correct, broken, or weird) remains mine.

*The only RoboTorq Gospel:*

`1 JouleTorq = 1 joule x 1 token / 1 second`
`1 TokenTorq = 3600 JouleTorq`
`1 RoboTorq = 1000 Tokentorq = 3.6 million JouleTorq`

## Persistence

SQLite is the default backend for development and readiness.

- Quick guide: see `docs/PERSISTENCE_SQLITE.md` (defaults, `kv` bootstrap, schema version repair).
- Commons persistence README: `crates/commons/src/util/persistence/README.md`.
- To run SQLite tests:

```powershell
cargo test -p commons --features "persistence"
```

Postgres and Testcontainers integrations are feature-gated and can be enabled later.

## Developer: OTLP (local smoke test)

If you want to exercise tracing locally (no CI required), the workspace includes an example and a small local OpenTelemetry Collector compose file.

Quick steps (PowerShell):

```powershell
# Start the local collector
docker compose -f .\ci\otlp-collector\docker-compose.yml up -d

# Run the example that emits a test span (enable OTLP feature)
cargo run --bin emit_traces --features otlp --release

# Optional: run the pre-check script which starts the collector, runs the example, collects logs, and tears down
python .\scripts\run_otlp_precheck.py --post-wait 8
```

Docs:
- Local OTLP setup: `docs/OTLP_LOCAL_SETUP.md`
- Message envelope & propagation guidance: `docs/MESSAGE_ENVELOPE.md`

Notes:
- The OTLP export is feature-gated on the `commons` crate (`otlp` feature). Examples and services must enable it to export traces.
- The CI e2e workflow for OTLP is intentionally kept off the default branch while we iterate on stability.