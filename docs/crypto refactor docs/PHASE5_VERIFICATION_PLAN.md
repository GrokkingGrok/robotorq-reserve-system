# Phase 5: Verification & Dispute Resolution

**Branch**: `feature/phase5-verification`  
**Date**: November 16, 2025  
**Status**: 🚀 READY TO START

---

## Overview

Phase 5 implements **cryptographic verification** and **merkle proof validation** across the entire proof chain:

```
Digger Hash → Falcon-1024 Signature → Refinery Merkle Tree → SPHINCS+ Signature → Mint Ledger
     ↓              ↓                        ↓                      ↓                ↓
  [VERIFY]      [VERIFY]                [VERIFY]              [VERIFY]          [VERIFY]
```

**Core Principle**: "Trust, but verify" - every signature must be cryptographically validated before accepting data.

---

## Phase 4 vs Phase 5

### Phase 4 (Complete ✅)
- **What**: Generate and transmit signatures
- **Digger**: Creates Falcon-1024 signatures on hash batches
- **Mint**: Creates SPHINCS+ signatures on Phase3 units
- **Focus**: Signature **creation** and **transmission**

### Phase 5 (This Phase)
- **What**: Verify and validate signatures
- **Refinery**: Verifies Falcon-1024 signatures from Digger
- **Mint**: Verifies SPHINCS+ signatures (if chaining)
- **Network**: API to verify merkle proofs
- **Focus**: Signature **verification** and **dispute resolution**

---

## Implementation Tasks

### 1. Refinery: Falcon-1024 Signature Verification ⚙️

**File**: `src/refinery/internal/crypto/falcon.go`

**Current State**:
```go
func VerifyHashBatch(...) error {
    // TODO: Implement Falcon-1024 verification
    // Production must verify cryptographic signature validity.
    return nil
}
```

**Implementation**:
```go
func VerifyHashBatch(publicKey []byte, signature []byte, hashBatch []string) error {
    // 1. Import pqcrypto-falcon library (via CGO or Go wrapper)
    // 2. Deserialize public key (1793 bytes for Falcon-1024)
    // 3. Reconstruct message: concatenate all hashes
    message := strings.Join(hashBatch, "")
    
    // 4. Verify signature using Falcon-1024
    valid := falcon.Verify(publicKey, []byte(message), signature)
    if !valid {
        return errors.New("invalid Falcon-1024 signature")
    }
    
    return nil
}
```

**Metrics to Add**:
- `refinery_falcon_signatures_verified_total` (counter)
- `refinery_falcon_signatures_failed_total` (counter)
- `refinery_falcon_verification_duration_seconds` (histogram)

**Integration Point**:
- Call `VerifyHashBatch()` in `ore_receiver.go` BEFORE accepting ore
- Reject and log invalid signatures (potential slashing event)

---

### 2. Mint: SPHINCS+ Signature Verification ⚙️

**File**: `src/mint/internal/crypto/sphincs.go`

**Current State**:
```go
func VerifyPhase3Unit(publicKey []byte, signature []byte, unit *Phase3RoboTorqUnit) error {
    // TODO: Implement SPHINCS+ verification
    return nil
}
```

**Implementation**:
```go
func VerifyPhase3Unit(publicKey []byte, signature []byte, unit *Phase3RoboTorqUnit) error {
    // 1. Use github.com/open-quantum-safe/liboqs-go
    // 2. Reconstruct message from unit fields
    message := fmt.Sprintf("%s|%s|%d|%s",
        unit.UnitID,
        unit.MerkleRoot,
        unit.IngotCount,
        unit.Timestamp.Format(time.RFC3339))
    
    // 3. Verify SPHINCS+ signature
    sig := oqs.Signature{}
    defer sig.Clean()
    
    if err := sig.Init("SPHINCS+-SHAKE-256f-simple", nil); err != nil {
        return err
    }
    
    valid, err := sig.Verify([]byte(message), signature, publicKey)
    if err != nil || !valid {
        return errors.New("invalid SPHINCS+ signature")
    }
    
    return nil
}
```

**Metrics to Add**:
- `mint_sphincs_signatures_verified_total`
- `mint_sphincs_signatures_failed_total`
- `mint_sphincs_verification_duration_seconds`

**Integration Point**:
- Verify in `phase3_robotorq_unit_assembler.go` when creating units
- Store verification timestamp in Phase3RoboTorqUnit

---

### 3. Merkle Proof Verification API 🌐

**New File**: `src/mint/internal/api/verify.go`

**Purpose**: Allow anyone to verify the proof chain from JTU → RT Unit

**Endpoints**:

#### `GET /verify/jtu/:hash`
Verify a JouleTorqUnit hash exists in the proof chain.

**Request**:
```bash
GET /verify/jtu/abc123...def
```

**Response**:
```json
{
  "exists": true,
  "ingot_id": "ingot-20251116-120000",
  "merkle_proof": [
    "hash1", "hash2", "hash3"  // Merkle branch
  ],
  "robotorq_unit_id": "rt-unit-001",
  "verified": true
}
```

#### `GET /verify/ingot/:id`
Verify an ingot and get its merkle proof.

**Response**:
```json
{
  "ingot_id": "ingot-20251116-120000",
  "merkle_root": "abc123...",
  "jtu_count": 3600,
  "robotorq_unit": "rt-unit-001",
  "signature_valid": true,
  "proof": ["hash1", "hash2", "hash3"]
}
```

#### `POST /verify/proof`
Verify a complete merkle proof.

**Request**:
```json
{
  "leaf_hash": "abc123...",
  "merkle_root": "def456...",
  "proof": ["hash1", "hash2", "hash3"],
  "leaf_index": 1234
}
```

**Response**:
```json
{
  "valid": true,
  "verified_at": "2025-11-16T12:00:00Z"
}
```

---

### 4. Merkle Tree Implementation 🌳

**New File**: `src/mint/internal/merkle/tree.go`

**Purpose**: Build and verify merkle trees from ingot hashes

```go
package merkle

import (
    "crypto/sha256"
    "encoding/hex"
)

type MerkleTree struct {
    Leaves []string
    Nodes  [][]string  // nodes[0] = leaves, nodes[height] = root
}

func NewMerkleTree(leaves []string) *MerkleTree {
    tree := &MerkleTree{Leaves: leaves}
    tree.build()
    return tree
}

func (t *MerkleTree) build() {
    currentLevel := t.Leaves
    t.Nodes = append(t.Nodes, currentLevel)
    
    for len(currentLevel) > 1 {
        nextLevel := []string{}
        
        for i := 0; i < len(currentLevel); i += 2 {
            left := currentLevel[i]
            right := left  // Duplicate if odd number
            if i+1 < len(currentLevel) {
                right = currentLevel[i+1]
            }
            
            // Hash(left || right)
            h := sha256.New()
            h.Write([]byte(left + right))
            parent := hex.EncodeToString(h.Sum(nil))
            
            nextLevel = append(nextLevel, parent)
        }
        
        t.Nodes = append(t.Nodes, nextLevel)
        currentLevel = nextLevel
    }
}

func (t *MerkleTree) Root() string {
    if len(t.Nodes) == 0 {
        return ""
    }
    return t.Nodes[len(t.Nodes)-1][0]
}

func (t *MerkleTree) GetProof(leafIndex int) []string {
    proof := []string{}
    index := leafIndex
    
    for level := 0; level < len(t.Nodes)-1; level++ {
        nodes := t.Nodes[level]
        
        // Get sibling
        var sibling string
        if index%2 == 0 {
            // Left node - sibling is right
            if index+1 < len(nodes) {
                sibling = nodes[index+1]
            } else {
                sibling = nodes[index]  // Duplicate
            }
        } else {
            // Right node - sibling is left
            sibling = nodes[index-1]
        }
        
        proof = append(proof, sibling)
        index = index / 2
    }
    
    return proof
}

func VerifyProof(leafHash string, proof []string, root string) bool {
    current := leafHash
    
    for _, sibling := range proof {
        h := sha256.New()
        // Assume leaf is always on left (can add position tracking)
        h.Write([]byte(current + sibling))
        current = hex.EncodeToString(h.Sum(nil))
    }
    
    return current == root
}
```

**Tests**: `merkle_test.go`
```go
func TestMerkleTree(t *testing.T) {
    leaves := []string{"a", "b", "c", "d"}
    tree := NewMerkleTree(leaves)
    
    root := tree.Root()
    assert.NotEmpty(t, root)
    
    // Verify proof for leaf index 2 ("c")
    proof := tree.GetProof(2)
    valid := VerifyProof("c", proof, root)
    assert.True(t, valid)
}
```

---

### 5. Dispute Resolution Protocol 🔨

**New File**: `src/trust/internal/dispute/resolver.go`

**Purpose**: Handle challenges to invalid proofs

**Flow**:
1. **Challenge**: Anyone submits invalid proof claim
2. **Verification**: Trust service verifies the claim
3. **Slashing**: If valid challenge, slash Digger's stake
4. **Reward**: Challenger receives portion of slashed stake

**API Endpoint**: `POST /dispute/challenge`

**Request**:
```json
{
  "jtu_hash": "abc123...",
  "claimed_ingot": "ingot-001",
  "proof": ["hash1", "hash2"],
  "reason": "Invalid signature"
}
```

**Response**:
```json
{
  "dispute_id": "dispute-001",
  "status": "pending_verification",
  "challenger": "wallet-xyz",
  "bounty": 0.001  // RT units if challenge valid
}
```

**Resolution States**:
- `pending_verification`: Trust verifying claim
- `valid_challenge`: Digger slashed, challenger rewarded
- `invalid_challenge`: Challenger loses small fee
- `resolved`: Dispute closed

---

## Implementation Order

### Sprint 1: Core Verification (Week 1)
1. ✅ Implement `VerifyHashBatch()` in Refinery (Falcon-1024)
2. ✅ Add verification call to `ore_receiver.go`
3. ✅ Add metrics for verification
4. ✅ Write unit tests

### Sprint 2: Mint Verification (Week 1)
1. ✅ Implement `VerifyPhase3Unit()` in Mint (SPHINCS+)
2. ✅ Add verification to Phase3 assembler
3. ✅ Add metrics
4. ✅ Write unit tests

### Sprint 3: Merkle Proofs (Week 2)
1. ✅ Implement `merkle.Tree` with proof generation
2. ✅ Add proof verification logic
3. ✅ Store merkle proofs in Mint database
4. ✅ Write comprehensive tests (3.6M leaf tree)

### Sprint 4: Verification API (Week 2)
1. ✅ Implement `/verify/*` endpoints
2. ✅ Add API documentation
3. ✅ Create integration tests
4. ✅ Test with real proof chains

### Sprint 5: Dispute Resolution (Week 3)
1. ✅ Implement dispute submission
2. ✅ Add verification logic
3. ✅ Implement slashing mechanism
4. ✅ Test dispute flow end-to-end

---

## Testing Strategy

### Unit Tests
```go
// Test Falcon verification with valid signature
func TestVerifyHashBatch_Valid(t *testing.T) {
    keypair := GenerateFalconKeypair()
    hashes := []string{"hash1", "hash2", "hash3"}
    message := strings.Join(hashes, "")
    signature := falcon.Sign(keypair.SecretKey, []byte(message))
    
    err := VerifyHashBatch(keypair.PublicKey, signature, hashes)
    assert.NoError(t, err)
}

// Test Falcon verification with invalid signature
func TestVerifyHashBatch_Invalid(t *testing.T) {
    keypair := GenerateFalconKeypair()
    hashes := []string{"hash1", "hash2", "hash3"}
    fakeSignature := make([]byte, 1302)  // Wrong signature
    
    err := VerifyHashBatch(keypair.PublicKey, fakeSignature, hashes)
    assert.Error(t, err)
}

// Test merkle proof for 3.6M leaves
func TestMerkleTree_LargeScale(t *testing.T) {
    leaves := generateRandomHashes(3_600_000)
    tree := NewMerkleTree(leaves)
    
    // Verify random leaf
    index := 1_234_567
    proof := tree.GetProof(index)
    valid := VerifyProof(leaves[index], proof, tree.Root())
    assert.True(t, valid)
}
```

### Integration Tests
```python
# tests/integration/test_verification.py

async def test_refinery_rejects_invalid_falcon_signature():
    """Test that Refinery rejects hash batches with invalid Falcon signatures"""
    
    # Send ore with invalid signature
    invalid_ore = create_test_ore(signature="fake_signature_bytes")
    
    response = await nats_client.publish("refinery.ore", invalid_ore)
    
    # Wait for rejection
    await asyncio.sleep(2)
    
    # Check metrics
    metrics = await get_prometheus_metrics("refinery")
    assert metrics["refinery_falcon_signatures_failed_total"] == 1
    assert metrics["refinery_ore_rejected_total"] == 1
```

### E2E Tests
```python
# tests/e2e/test_full_verification_chain.py

async def test_full_proof_verification():
    """Test complete proof chain: Digger → Refinery → Mint → Verify API"""
    
    # 1. Digger creates signed hash batch
    digger_response = await execute_contract()
    assert digger_response["signature_valid"] == True
    
    # 2. Refinery verifies and assembles ingot
    await wait_for_ingot_assembly()
    ingot = await get_latest_ingot()
    assert ingot["falcon_signature_verified"] == True
    
    # 3. Mint creates Phase3 unit
    await wait_for_phase3_unit()
    unit = await get_latest_phase3_unit()
    assert unit["sphincs_signature_valid"] == True
    
    # 4. Verify proof via API
    jtu_hash = digger_response["jtu_hashes"][0]
    verify_response = await verify_jtu(jtu_hash)
    
    assert verify_response["exists"] == True
    assert verify_response["verified"] == True
    assert len(verify_response["merkle_proof"]) > 0
```

---

## Metrics Dashboard (Grafana)

### New Panel: Signature Verification Rate

**Query**:
```promql
rate(refinery_falcon_signatures_verified_total[5m])
```

**Visualization**: Time series graph
- Green: Verified signatures
- Red: Failed signatures

### New Panel: Verification Latency

**Query**:
```promql
histogram_quantile(0.99, 
  rate(refinery_falcon_verification_duration_seconds_bucket[5m])
)
```

**Alert**: p99 latency > 100ms

---

## Security Considerations

### Slashing Parameters
- **Invalid Signature**: Slash 10% of stake
- **Repeated Offenses**: Exponential increase (10%, 25%, 50%, 100%)
- **Challenge Reward**: 20% of slashed amount
- **Minimum Stake**: 0.001 RT to prevent spam

### Rate Limiting
- **Verification API**: 100 requests/minute per IP
- **Dispute Submission**: 10 disputes/hour per wallet
- **Proof Queries**: 1000 queries/minute globally

### DoS Protection
- Cache merkle proofs for frequently queried JTUs
- Reject proofs with >1000 depth (too expensive to verify)
- Implement proof-of-work for dispute submissions

---

## Success Criteria

- ✅ Refinery verifies 100% of Falcon-1024 signatures before accepting ore
- ✅ Mint verifies SPHINCS+ signatures on Phase3 units
- ✅ Verification API responds in <50ms for 99% of requests
- ✅ Merkle tree supports 3.6M leaves with <1GB memory
- ✅ Dispute resolution completes in <5 minutes
- ✅ Slashing mechanism tested with invalid signatures
- ✅ Grafana dashboard shows verification metrics real-time
- ✅ E2E test proves full chain verification working

---

## Next Steps After Phase 5

### Phase 6: Scalability
- TOON binary encoding (60% bandwidth reduction)
- Geographic sharding (1M+ robots)
- Compression for archival storage
- Cloud backup integration

### Phase 7: Network Layer
- P2P gossip protocol for RT unit distribution
- DHT for merkle proof storage
- Decentralized dispute resolution voting
- Cross-shard proof verification

---

**Ready to start Phase 5?** Let's implement signature verification! 🚀
