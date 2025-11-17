# Currency Refactor - Key Design Principles

## **CRITICAL**: What Mint Cares About

The Mint **ONLY** cares about **HASHES** (cryptographic proofs), NOT totals or aggregated values.

### Current State (will be cleaned in `feature/crypto-refactor`):
- ❌ `TokenTorqIngot.JouleTorqTotal` - REDUNDANT (calculable from units)
- ❌ `TokenTorqIngot.RoboStakeTotal` - REDUNDANT (calculable from units)
- ❌ `RoboTorqUnit.TotalJoules` - REDUNDANT (calculable from ingots)
- ❌ `RoboTorqUnit.TotalRoboStake` - REDUNDANT (calculable from ingots)

### What Actually Matters:
- ✅ `JouleTorqUnit[]` - Array of 3,600 atomic token proofs
- ✅ `JouleTorqUnit.Hash` - SHA256 of each unit's data
- ✅ `TokenTorqIngot.BranchHash` - Merkle hash of 3,600 unit hashes
- ✅ `RoboTorqUnit.MerkleRoot` - Root hash of 1,000 ingot branch hashes

**Proof Chain**: Token → Unit Hash → Ingot Branch Hash → RT Merkle Root

All original **ore data is DISCARDED** after unit extraction. Only units and their hashes persist through the pipeline.

---

## **PHYSICS CLARIFICATION**: Energy vs Tokens

### What is a JouleTorqUnit?

**1 JouleTorqUnit = 1 TOKEN** (NOT 1 joule!)

A unit represents ONE token's worth of computational work, and that unit *contains* the joules consumed during that token's creation.

### Example: 2 kW Mining Rig, 10 tokens/second

**In 1 second:**
```
Power: 2000 W
Energy: 2000 W × 1 sec = 2000 Joules
Tokens: 10 tokens/sec × 1 sec = 10 tokens
JouleTorqUnits created: 10 units (1 per token)
Joules per token: 2000 J ÷ 10 = 200 J/token
```

**Each JouleTorqUnit:**
```json
{
  "token_id": "contract-m5-t0007",
  "joules_consumed": 200,
  "robo_stake_paid": 0.0001388
}
```

**In 1 hour (3600 seconds):**
```
Tokens: 10 tokens/sec × 3600 sec = 36,000 tokens
JouleTorqUnits: 36,000 units (1 per token)
Total energy: 2000 W × 3600 sec = 7,200,000 Joules (7.2 MJ)
```

### Ingot Assembly (3,600 units → 1 ingot)

**From 1 hour of mining (2 kW, 10 tokens/sec):**
```
36,000 units produced
÷ 3,600 units per ingot
= 10 ingots created
Leftover: 0 units (perfect division)
```

**If ore arrives with 300 tokens and accumulator has 3,500 units:**
```
Total: 3,500 + 300 = 3,800 units
Build ingot: 3,600 units (first ingot complete)
Leftover: 200 UNITS (not joules!) carried to next ingot
```

### RoboTorqUnit Assembly (1,000 ingots → 1 RT)

**Time to create 1 RT (2 kW, 10 tokens/sec):**
```
1,000 ingots × 3,600 units = 3,600,000 units needed
36,000 units/hour ÷ 3,600,000 units = 100 hours
```

---

## **KEY INSIGHT**: Never "Leftover Joules"

**There are NEVER leftover joules!**

Why? Because:
1. Ore contains **whole tokens** (e.g., 300 tokens)
2. Each token becomes **exactly 1 unit**
3. Joules are **divided evenly** across tokens during unit extraction
4. Units **accumulate** until 3,600 → 1 ingot
5. Only **leftover UNITS** (not joules) carry to next ingot

**Ore Processing Flow:**
```
JouleTorqOre (300 tokens, 60,000 joules)
  ↓
Extract Units (divide joules evenly):
  - 60,000 J ÷ 300 tokens = 200 J/token
  - Create 300 JouleTorqUnits (each with 200 J)
  ↓
Add to Queue (300 units)
  ↓
IngotAssembler accumulates units
  - Has 3,500 units
  - Receives 300 units
  - Total: 3,800 units
  ↓
Build ingot from first 3,600 units
  ↓
Leftover: 200 UNITS (NOT joules!)
```

Joules exist **only within units** - they never accumulate separately.

---

## Data Flow Summary

```
Digger mines token
  → Creates JouleTorqOre (300 tokens, N joules)
  → Sends to Refinery

Refinery receives ore
  → Extracts 300 JouleTorqUnits (1 per token)
  → DISCARDS ore (ephemeral, GC'd after 1ms)
  → Queues units

IngotAssembler
  → Accumulates units until 3,600 total
  → Creates TokenTorqIngot (3,600 units)
  → Calculates BranchHash (merkle tree of unit hashes)
  → Units preserved, ore forgotten

Mint receives ingot
  → Validates BranchHash matches unit hashes
  → Accumulates 1,000 ingots
  → Creates RoboTorqUnit (1 RT)
  → Calculates MerkleRoot (tree of branch hashes)
  → Only hashes matter, totals are redundant

Final State: 1 RT
  → 1,000 ingots × 3,600 units = 3,600,000 units
  → Complete merkle tree proof
  → Can verify any token back to RT
  → Total data: ~170 GB (vs 98 TB if we kept ore)
```

---

## Next Steps (feature/crypto-refactor)

1. **Remove redundant total fields**:
   - TokenTorqIngot: Remove `JouleTorqTotal`, `RoboStakeTotal`
   - RoboTorqUnit: Remove `TotalJoules`, `TotalRoboStake`

2. **Implement Falcon-1024 signatures**:
   - Sign ore merkle roots in Digger
   - Verify signatures in Refinery
   - Sign ingot branch hashes in Refinery
   - Verify signatures in Mint

3. **Optional: SPHINCS+ archival signatures**:
   - Sign RoboTorqUnit merkle root for "forever proof"

4. **Update logging**:
   - Keep totals in log output (useful for debugging)
   - But don't store them in structs
   - Calculate on-the-fly from units when needed

---

**Remember**: RoboTorq is about **PROOF**, not accounting. We care about cryptographic verification (hashes), not aggregated totals. The physics (joules, tokens) lives in the units; everything above is just proof trees.

*"The token is the atom, the hash is the bond, the merkle tree is the molecule."* 🔬⚡
