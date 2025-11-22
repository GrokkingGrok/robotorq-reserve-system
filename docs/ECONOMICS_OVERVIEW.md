# RoboTorq Economics Overview

This captures the deeper monetary mechanics removed from the shortened README.

## 1. Equation of Intelligence-Value Exchange
`RoboTorq_out = RoboTorq_in × Torq_Factor`

Torq_Factor = (Value of output in RT) / (Raw robotic labor cost in RT).
It models markup / efficiency / premium embedding (e.g., craftsmanship, scarcity of robot type).

## 2. Compression & Issuance Flow
| Step | Input | Operation | Output | Economic Meaning |
|------|-------|-----------|--------|------------------|
| 1 | Token work (joules + signature) | Hash + Sign | JouleTorqUnit | Atomic priced effort |
| 2 | 3,600 units | Merkle branch root | TokenTorqIngot | Time‑slice compression |
| 3 | 1,000 ingots | Merkle batch root + Validation | RoboTorq Certificate | Reserve issuance basis |
| 4 | Certificates | Scheduling + Demurrage | Streams / UBD | Circulation & redistribution |

## 3. Vault Subsystems (Shadow Reserve)
| Vault | Primary Function | Flows | Notes |
|-------|------------------|-------|-------|
| StakeVault | Provides stake constraint for labor contracts | Stake → Contract → Return | Prevents uncontrolled inflation |
| CertVault | Holds certificate proofs & timing metadata | Certificate → Schedule | Basis for UBD & demurrage math |
| DistoVault | Executes UBD payments & balance adjustments | Schedule → Wallet/ShortVault | Enforces continuous circulation |

ShortVaults = Liquid member balances. Long‑term reserve pools remain locked to guarantee ongoing issuance integrity.

## 4. Distribution Activation Path
Physical bearer RT (printed bonds) → Scan → Wallet bootstrap → Minimum internal spend threshold (e.g. 750 / 1000 RT) → UBD eligibility. Ensures real economic participation precedes perpetual dividend entitlement.

## 5. BidNet Role
Decentralized marketplace / labor exchange: supports converting RoboTorq into goods, funding new robotic labor contracts, optional external stable asset (e.g. USDC) swaps among activated participants—without undermining the physics basis.

## 6. Value Principles
- Energy & computation are measurable and anti‑fraud friendly.
- Merkle compression yields scalable auditability (proof chain remains verifiable while historical raw units can be pruned).
- Demurrage: discourages passive hoarding; funds security, maintenance, development.
- Stake cycle prevents runaway contract issuance disconnected from reserve backing.

## 7. Risk & Integrity Considerations
- Incorrect energy measurement → mispriced units → economic drift (mitigate by pricing on registered, verfied capacity).
- Signature compromise → forged units: detect by hash mismatch at ingestion layer & periodic batch audits.
- Demurrage model misconfiguration → either stagnation or punitive churn; parameter governance must be transparent.

## 8. References
- Mint Architecture: `src/mint/MINT_ARCHITECTURE.md`
- Refinery Architecture: `src/refinery/REFINERY_ARCHITECTURE.md`
- Vault Final Design: `src/vault/Architecture/Final design/VAULT_MVP_FINAL_DESIGN.md`
- Proof Chain: `docs/PROOF_CHAIN_ARCHITECTURE.md`

Document purpose: conceptual economics snapshot—implementation details live in service‑specific docs.