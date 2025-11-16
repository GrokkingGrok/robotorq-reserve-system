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
- Power consumption: 2000 W
- JouleTorqs produced: 10 tokens × 2000 W = 20,000 JTU/sec (CROSS PRODUCT!)

For each of 20,000 JTU:
1. Hash token data + energy
2. Sign hash
3. Queue for batch send
4. Discard raw joules

After 60 seconds:
- JTUs created: 20,000 JTU/sec × 60 sec = 1,200,000 JTU
- Batch size: 1.2M hashes × 32 bytes = 38.4 MB
- Send to Refinery

LOCAL STORAGE (if Digger stores proofs):
- Full JTUs: 1.2M × 450 bytes = 540 MB per minute
- Hashes only: 1.2M × 32 bytes = 38.4 MB per minute
- 30 days: 1.66 TB (full) or 55 GB (hashes only)
```

### Hash-Only Transmission (Critical Optimization)

**Problem**: Sending full JTUs wastes 93% of bandwidth

```
Full JTU transmission:
- 20,000 JTU/sec × 450 bytes = 9 MB/sec
- Over 24 hours: 777 GB/day (expensive!)

Hash-only transmission:
- 20,000 hashes/sec × 32 bytes = 640 KB/sec  
- Over 24 hours: 55 GB/day (affordable!)
- Bandwidth savings: 93% reduction ✅
```

**Architecture Decision**: Digger stores JTUs locally, sends only hashes

```
Digger responsibilities:
1. Create JTU (hash + signature + metadata)
2. Store JTU locally (SQLite or flat file)
3. Send ONLY hash to Refinery (32 bytes)
4. Keep JTUs for 30-day dispute window
5. Provide merkle proofs on demand

Refinery responsibilities:
1. Receive hash batches from Diggers
2. Build merkle trees from hashes
3. NO NEED for full JTU data!
```

### Local Storage Requirements

| Robot Power | Tokens/sec | JTU/sec | 30-day Storage (Full) | 30-day Storage (Hashes) |
|-------------|-----------|---------|----------------------|------------------------|
| 100W        | 1         | 100     | 117 GB               | 743 MB                 |
| 1kW         | 10        | 10,000  | 11.7 TB              | 74.3 GB                |
| 2kW         | 10        | 20,000  | 23.3 TB              | 148.6 GB               |
| 10kW        | 100       | 1,000,000 | 1.17 PB            | 7.4 TB                 |

**Solution**: Use compression + incremental archival

```
Storage strategy:
1. Days 0-7: Uncompressed local SSD (hot data)
2. Days 8-30: Compressed archive (gzip/zstd saves 70%)
3. After 30 days: Move to cheap cloud storage (Backblaze B2)
4. After 1 year: Prune to hashes only (delete full JTUs)
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

### Phase 1: Digger Rewrite (feature/digger-refactor) **← START HERE**
**Goal**: Pure HTTP server, no Tauri, local storage, hash-only transmission

- [ ] Strip out Tauri frontend completely
- [ ] Keep HTTP server for contract management
- [ ] Implement SQLite storage for JTUs (per contract)
- [ ] Implement JTU hash generation (SHA256 of token + joules)
- [ ] Add placeholder signatures (Falcon-1024 later)
- [ ] Create hash-only NATS messages to Refinery
- [ ] Implement 30-day cleanup job (prune old JTUs)
- [ ] Add compression (gzip) for archived JTUs
- [ ] Stake gating: only send hashes after RoboStake paid

### Phase 2: Refinery Hash Receiver (feature/proof-chain-refinery)
- [ ] Update NATS subscriber to receive hash batches (not full JTUs)
- [ ] Change unit extraction: `count = tokens × joules` (cross product)
- [ ] Implement 3.6M hash counting (not 3600 joule threshold)
- [ ] Build merkle tree from JTU hashes
- [ ] Sign ingot branch_hash with Falcon-1024 (placeholder for now)
- [ ] Discard JTU hashes after merkle tree built
- [ ] Remove JouleTorqTotal/RoboStakeTotal from ingots

### Phase 3: Mint Merkle Trees (feature/proof-chain-mint)
- [ ] Remove Ingots[] array from RoboTorqUnit
- [ ] Build merkle tree from 1,000 ingot branch_hashes
- [ ] Sign RT merkle_root with SPHINCS+ (archival, placeholder for now)
- [ ] Implement Ledger storage (PostgreSQL with JSONB for merkle trees)
- [ ] Broadcast only RT hash + signature to network
- [ ] Remove TotalJoules/TotalRoboStake from RoboTorqUnit

### Phase 4: Cryptography (feature/phase4-crypto) ✅ COMPLETE
- [x] Replace placeholder signatures with real Falcon-1024 (Rust: `pqcrypto-falcon`)
- [x] Add SPHINCS+ for Mint archival signatures (Go: `github.com/open-quantum-safe/liboqs-go`)
- [x] Implement Phase2 batch sender (Refinery → Mint via NATS)
- [x] Fix Prometheus scraping (Mint metrics on port 9090)
- [x] Create Grafana crypto pipeline dashboard
- [ ] Implement signature verification in Refinery/Mint (deferred to Phase 5)
- [ ] Add slashing for invalid signatures (deferred to Phase 5)

**Status**: Core crypto implemented and flowing. Dashboard shows real-time signatures.  
**Validation issue**: Mint rejecting Phase2Ingots (expects Phase1 format) - tracked separately.

### Phase 5: Verification & Dispute (feature/proof-chain-verify)
- [ ] Implement merkle proof verification
- [ ] Allow anyone to verify: JTU → Ingot → RT
- [ ] Create verification API endpoint
- [ ] Add proof visualization tools
- [ ] Implement dispute resolution protocol

### Phase 6: Scalability (feature/scalability)
- [ ] **TOON Encoding**: Replace JSON with binary TOON (60% bandwidth savings)
- [ ] **Sharding**: Split Refinery into geographic shards (>1M robots)
- [ ] **Compression**: Add zstd compression for archival storage
- [ ] **Cloud Archival**: Integrate Backblaze B2 for old JTUs

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
