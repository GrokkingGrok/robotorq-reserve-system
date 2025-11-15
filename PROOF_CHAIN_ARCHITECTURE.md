# RoboTorq Proof Chain Architecture (CORRECT)

**Date**: November 15, 2025  
**Status**: 🔴 CRITICAL DISCOVERY - Current implementation is WRONG  
**Next Branch**: `feature/proof-chain-refactor`

---

## 🚨 The Problem with Current Implementation

**Current code** treats JouleTorqUnit as a data structure with joules/tokens stored in each unit.

**Reality**: JouleTorqUnit should be a **PROOF** (hash + signature), not a data container.

**Economic bug**: Current code creates units proportional to tokens only, ignoring power consumption!
- Robot A: 10 tok/s @ 2kW should get 20,000 JTU/sec
- Robot B: 10 tok/s @ 100W should get 1,000 JTU/sec
- **Current code gives both 10 units/sec** ❌

---

## ✅ Correct Architecture: Hash-Based Proof Chain

### Core Principle
**"Data flows down, proofs flow up, only hashes persist"**

```
Digger:   Raw data (joules, tokens) → Hash → Sign → Discard data
Refinery: JTU hashes → Merkle tree → Sign → Discard JTU hashes  
Mint:     Ingot hashes → Merkle tree → Sign → Store in ledger
Network:  Only final RT hash broadcast
```

---

## Stage 1: Digger (Proof Generation)

### What Happens
For **each token** generated during work:
1. **Measure**: Joules consumed for this specific token
2. **Hash**: `SHA256(token_id || joules || timestamp || contract_id || digger_id)`
3. **Sign**: Dilithium/Falcon signature on hash
4. **Create JTU**: `{hash, signature, metadata}`
5. **DISCARD joules**: Already captured in hash, no longer needed

### Time-Based Batching
Every **1 minute** (configurable):
- Send batch of all JTUs created in that window to Refinery
- **Variable batch size**: Could be 100 JTUs or 1,000,000 JTUs depending on robot performance
- Batch = `[JTU₁, JTU₂, ..., JTUₙ]`

### JouleTorqUnit Structure (Corrected)
```rust
// Digger (Rust)
pub struct JouleTorqUnit {
    pub hash: String,          // SHA256(token_id + joules + timestamp + ...)
    pub signature: Vec<u8>,    // Dilithium/Falcon signature
    pub digger_id: String,     // Who created this proof
    pub contract_id: String,   // What contract
    pub timestamp: i64,        // Unix timestamp
    // NO joules field! Already hashed and discarded
    // NO token_id field! Already in hash
}
```

### Example: 2kW Robot, 10 tokens/sec
```
In 1 second:
- Tokens generated: 10
- Joules consumed: 2000 J
- JouleTorqs produced: 10 tokens × 2000 J = 20,000 JTU

For each of 20,000 JTU:
1. Hash token data + energy
2. Sign hash
3. Queue for batch send
4. Discard raw joules

After 60 seconds:
- JTUs created: 20,000 JTU/sec × 60 sec = 1,200,000 JTU
- Send batch of 1.2M JTU hashes to Refinery
```

---

## Stage 2: Refinery (Ingot Assembly)

### What Happens
1. **Receive**: Variable-size batches of JTUs from all diggers
2. **Queue**: All JTU hashes in order received (multi-digger queue)
3. **Accumulate**: Count to 3,600,000 JTU hashes
4. **Build Merkle Tree**: Binary tree from 3.6M JTU hashes
5. **Calculate Branch Hash**: Merkle root of 3.6M hashes
6. **Sign Ingot**: Refinery signs the branch_hash
7. **DISCARD JTU hashes**: Already compressed into merkle tree
8. **Store Ingot**: `{ingot_id, branch_hash, signature}`

### Ingot Batching
Accumulate **1,000 ingots** → Send batch to Mint

### TokenTorqIngot Structure (Corrected)
```go
// Refinery (Go)
type TokenTorqIngot struct {
    IngotID      string    // ingot-20251115-001
    BranchHash   string    // Merkle root of 3.6M JTU hashes
    Signature    []byte    // Refinery's Falcon signature
    RefineryID   string    // Which refinery created this
    Timestamp    time.Time // When assembled
    // NO Units[] array! JTU proofs discarded after merkle tree built
    // NO JouleTorqTotal! Not needed, just counting units
    // NO RoboStakeTotal! Not needed, just counting units
}
```

### Example Flow
```
Refinery receives JTUs from 100 diggers:
- Digger A: 1,200,000 JTU (2kW, 10 tok/sec, 60 sec)
- Digger B: 60,000 JTU (100W, 10 tok/sec, 60 sec)
- Digger C: 300,000 JTU (500W, 10 tok/sec, 60 sec)
- ... (97 more diggers)

Total received: 50,000,000 JTU hashes (variable)

Ingot assembly:
- Ingot 1: First 3,600,000 JTU hashes → merkle tree → branch_hash_1
- Ingot 2: Next 3,600,000 JTU hashes → merkle tree → branch_hash_2
- ...
- Ingot 13: Next 3,600,000 JTU hashes → merkle tree → branch_hash_13
- Leftover: 3,200,000 JTU hashes (wait for more digger batches)

Send batch of 13 ingots to Mint when ready
```

### Key Insight: No "Leftover Joules"
There are **NEVER** leftover joules because:
1. JTU count = tokens × joules (integer cross product)
2. Only **leftover JTU hashes** exist (not joules)
3. Leftover hashes wait for next batch to reach 3.6M threshold

---

## Stage 3: Mint (RoboTorq Creation)

### What Happens
1. **Receive**: Batches of ingots from Refinery (variable count)
2. **Accumulate**: Count to 1,000 ingots
3. **Build Merkle Tree**: Binary tree from 1,000 ingot branch_hashes
4. **Calculate Merkle Root**: Root hash of 1,000 branch hashes
5. **Sign RT**: Mint signs the merkle_root (SPHINCS+ for archival)
6. **STORE in Ledger**: All ingot hashes + merkle tree structure
7. **BROADCAST**: Only `{RT_id, merkle_root, signature}` to network

### RoboTorqUnit Structure (Corrected)
```go
// Mint (Go)
type RoboTorqUnit struct {
    UnitID       string    // RT-20251115-001
    MerkleRoot   string    // Root of 1,000 ingot branch hashes
    Signature    []byte    // Mint's SPHINCS+ signature
    MintedAt     time.Time // UTC timestamp
    // NO Ingots[] array! Stored separately in ledger
    // NO TotalJoules! Not needed
    // NO TotalRoboStake! Not needed
}
```

### Ledger Storage
```go
type Ledger struct {
    RoboTorqUnits map[string]*RoboTorqUnit  // RT_id → RT
    IngotProofs   map[string]*IngotProof    // Ingot_id → {branch_hash, parent_RT}
    MerkleTrees   map[string]*MerkleTree    // RT_id → Complete tree structure
}

// Anyone can verify:
// JTU hash → Ingot branch_hash → RT merkle_root
// Using merkle proofs stored in ledger
```

---

## Proof Chain Visualization

```
┌─────────────────────────────────────────────────────────────┐
│ DIGGER: Raw Work → Proofs                                   │
├─────────────────────────────────────────────────────────────┤
│ Token 1: 200 J → Hash₁ → Sign₁ → JTU₁                      │
│ Token 2: 200 J → Hash₂ → Sign₂ → JTU₂                      │
│ ...                                                          │
│ Token 10: 200 J → Hash₁₀ → Sign₁₀ → JTU₁₀                  │
│ ────────────────────────────────────────────────────────────│
│ DISCARD: All joules (2000 J) - already in hashes            │
│ SEND: 20,000 JTU hashes (1 sec @ 2kW, 10 tok/s)            │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ REFINERY: JTU Hashes → Ingot                                │
├─────────────────────────────────────────────────────────────┤
│ Receive: 3,600,000 JTU hashes from multiple diggers         │
│ Build: Merkle tree (binary tree, 21 levels deep)            │
│ Root: Branch_Hash (32 bytes)                                │
│ Sign: Refinery signature on Branch_Hash                     │
│ ────────────────────────────────────────────────────────────│
│ DISCARD: 3.6M JTU hashes (115 MB) - compressed to 32 bytes  │
│ SEND: 1 Ingot {branch_hash, signature}                      │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ MINT: 1000 Ingots → RoboTorqUnit (1 RT)                    │
├─────────────────────────────────────────────────────────────┤
│ Receive: 1,000 Ingot branch_hashes                          │
│ Build: Merkle tree (binary tree, 10 levels deep)            │
│ Root: Merkle_Root (32 bytes)                                │
│ Sign: Mint signature (SPHINCS+ archival proof)              │
│ ────────────────────────────────────────────────────────────│
│ STORE: Ledger (all hashes + merkle trees) - 170 MB          │
│ BROADCAST: {RT_id, merkle_root, signature} - 5 KB           │
└─────────────────────────────────────────────────────────────┘
```

---

## Economic Formula (Correct)

### JouleTorq Production Rate
```
JouleTorqs/second = (tokens/sec) × (watts)

Example 1: 10 tok/s @ 2000 W
→ 10 × 2000 = 20,000 JTU/sec

Example 2: 10 tok/s @ 100 W  
→ 10 × 100 = 1,000 JTU/sec

Example 3: 100 tok/s @ 5000 W
→ 100 × 5000 = 500,000 JTU/sec
```

### RoboTorq Minting Rate
```
1 RT = 3,600,000 JTU

Example 1: 20,000 JTU/sec
→ 3,600,000 ÷ 20,000 = 180 sec = 3 minutes per RT

Example 2: 1,000 JTU/sec
→ 3,600,000 ÷ 1,000 = 3,600 sec = 60 minutes per RT

Example 3: 500,000 JTU/sec  
→ 3,600,000 ÷ 500,000 = 7.2 sec per RT
```

**Key Insight**: Higher power + higher throughput = more RT minted (proportional to BOTH)

---

## Storage Comparison

### Naive Approach (Storing Everything)
```
1 RT = 3,600,000 JTU × 500 bytes/JTU
     = 1.8 GB per RT ❌

1 year @ 1 RT/hour:
     = 1.8 GB × 24 × 365
     = 15.8 TB per year ❌❌❌
```

### Merkle Compression (Correct)
```
Layer 0: 3,600,000 JTU hashes × 32 bytes = 115 MB
Layer 1: 1,000 Ingot hashes × 32 bytes = 32 KB
Layer 2: 1 RT hash × 32 bytes = 32 bytes

Ledger Storage: 115 MB + merkle tree overhead (~230 MB total)
Network Broadcast: 32 bytes + signature (~5 KB)

1 year @ 1 RT/hour:
     = 230 MB × 24 × 365  
     = 2 TB per year ✅ (13× reduction from naive)
```

---

## Implementation Roadmap

### Phase 1: Digger (feature/proof-chain-digger)
- [ ] Implement JTU hash generation (SHA256 of token + joules)
- [ ] Add Falcon-1024 signing of JTU hashes
- [ ] Create time-based batching (send every 60 seconds)
- [ ] Remove joules from JTU after hashing
- [ ] Update ore structure to send array of JTU proofs

### Phase 2: Refinery (feature/proof-chain-refinery)
- [ ] Change unit extraction: `count = tokens × joules` (cross product)
- [ ] Remove JoulesConsumed field from JouleTorqUnit
- [ ] Implement 3.6M unit counting (not 3600 joule threshold)
- [ ] Build merkle tree from JTU hashes
- [ ] Sign ingot branch_hash with Falcon-1024
- [ ] Discard JTU hashes after merkle tree built
- [ ] Remove JouleTorqTotal/RoboStakeTotal from ingots

### Phase 3: Mint (feature/proof-chain-mint)
- [ ] Remove Ingots[] array from RoboTorqUnit
- [ ] Build merkle tree from 1,000 ingot branch_hashes
- [ ] Sign RT merkle_root with SPHINCS+ (archival)
- [ ] Implement Ledger storage (ingot hashes + merkle trees)
- [ ] Broadcast only RT hash + signature to network
- [ ] Remove TotalJoules/TotalRoboStake from RoboTorqUnit

### Phase 4: Verification (feature/proof-chain-verify)
- [ ] Implement merkle proof verification
- [ ] Allow anyone to verify: JTU → Ingot → RT
- [ ] Create verification API endpoint
- [ ] Add proof visualization tools

---

## Testing Strategy

### Unit Tests
```go
// Test cross product calculation
func TestJouleTorqProduction(t *testing.T) {
    tokens := 10
    joules := 2000
    expected := 20_000
    
    count := tokens * joules
    assert.Equal(t, expected, count)
}

// Test merkle tree construction
func TestMerkleTree3_6Million(t *testing.T) {
    hashes := generateRandomHashes(3_600_000)
    tree := BuildMerkleTree(hashes)
    
    // Verify root hash
    root := tree.Root()
    assert.NotEmpty(t, root)
    
    // Verify proof for random leaf
    leafIndex := 1_234_567
    proof := tree.GetProof(leafIndex)
    assert.True(t, VerifyProof(hashes[leafIndex], proof, root))
}
```

### Integration Tests
```
Test: Full pipeline with 2kW robot
1. Digger produces 20,000 JTU/sec
2. After 180 seconds: 3,600,000 JTU
3. Refinery builds 1 ingot
4. After 1,000 ingots: Mint creates 1 RT
5. Verify merkle proofs work end-to-end
```

---

## Migration Strategy

**DO NOT** try to migrate existing data. This is a fundamental architecture change.

1. Create new branch: `feature/proof-chain-refactor`
2. Implement Phase 1-4 in parallel feature branches
3. Integration test on clean testnet
4. Deploy to production (fresh start, no migration)

---

## Questions to Resolve

1. **JTU uniqueness**: How do we ensure each JTU hash is unique?
   - Include nonce/sequence number in hash?
   - Use timestamp with microsecond precision?

2. **Merkle tree implementation**: Binary tree or other structure?
   - Binary tree (Bitcoin-style) ✅
   - Handle odd counts by duplicating last hash

3. **Ledger database**: What stores the merkle trees?
   - PostgreSQL with JSONB for tree structure?
   - Dedicated graph database?
   - Flat files with indexed lookup?

4. **Proof verification**: Who can request proofs?
   - Public API (anyone can verify) ✅
   - Restricted to participants only?

5. **Signature scheme**: Falcon or Dilithium for ephemeral?
   - Falcon-1024 for Digger/Refinery (fast, compact) ✅
   - SPHINCS+ for Mint (archival, paranoid security) ✅

---

## Key Takeaways

1. **JouleTorq count = tokens × joules** (cross product, not 1:1 with tokens)
2. **Data flows down, proofs flow up** (compress at each stage)
3. **Only hashes persist** (joules, tokens, etc. discarded after hashing)
4. **Merkle trees enable verification** without storing all data
5. **No leftover joules** (only leftover JTU hashes waiting for next batch)
6. **Current implementation is fundamentally wrong** (treats units as data, not proofs)

---

**Next Step**: Create `feature/proof-chain-refactor` branch and start Phase 1 (Digger).

*"The proof is in the hash, not the data."* 🔐⚡
