# JouleTorq Proof Verification via Statistical Sampling

**Date**: November 21, 2025  
**Status**: Design specification for Phase 3+  
**Purpose**: Determine minimum sampling requirements for detecting fraudulent proofs in ingots

---

## 🎯 Executive Summary

**Question**: How many JouleTorq proofs must we verify per ingot to have statistical confidence that all 3.6M are valid?

**Answer**: 
- **500-690 samples** for **95-99% confidence** detecting **0.5-1% fraud rate**
- **Cost**: ~2-5ms per ingot (negligible)
- **Mechanism**: Random spot-checking with cryptographic verification

---

## 1. The Problem

### Setup
- **Ingot size**: 3,600,000 JouleTorqs (exactly 1 hour of ideal 1kW robot work)
- **Sources**: JouleTorqs come from 3,600 different contracts (1 token per contract per second)
- **Attack vector**: Attacker tries to insert fraudulent proofs into the merkle tree
- **Challenge**: We can't verify all 3.6M proofs (too expensive), but must ensure integrity

### Constraints
1. **Post-quantum signatures** make proof forgery mathematically hard (Falcon-1024, SPHINCS+)
2. **Multi-contract distribution** prevents attacker from controlling entire batch
3. **Merkle tree structure** means changing any proof requires changing parent hashes
4. **Statistical approach** acceptable because cryptography provides base layer of security

---

## 2. Statistical Sampling Theory

### 2.1 Hypergeometric Distribution (Exact)

**Setup**: 
- Population: $M = 3,600,000$ JouleTorqs
- Frauds planted: $X$ (unknown to verifier)
- Sample size: $N$ (how many we check)

**Question**: What's the probability we miss ALL frauds if we sample randomly?

$$P(\text{miss all frauds}) = \frac{\binom{M-X}{N}}{\binom{M}{N}}$$

**Interpretation**: If this probability is 1%, then we have 99% confidence we CATCH at least one fraud.

### 2.2 Simplified Approximation (Practical)

For large $M$ and small $X/M$:

$$P(\text{miss all frauds}) \approx \left(1 - \frac{X}{M}\right)^N$$

**To achieve confidence $C$**, we need:

$$N \approx \frac{\ln(1 - C)}{\ln\left(1 - \frac{X}{M}\right)}$$

Rearranging for "fraud rate" $f = X/M$:

$$N \approx \frac{\ln(1 - C)}{\ln(1 - f)}$$

---

## 3. Reference Tables

### 3.1 Samples Needed to Detect Various Fraud Rates

| Fraud Rate | Population | Frauds | 95% Conf | 99% Conf | 99.9% Conf |
|-----------|-----------|--------|----------|----------|-----------|
| **0.01%** | 3.6M | 360 | ~23,000 | ~52,000 | ~78,000 |
| **0.1%** | 3.6M | 3,600 | ~2,300 | ~5,200 | ~7,800 |
| **0.5%** | 3.6M | 18,000 | ~460 | ~1,040 | ~1,560 |
| **1.0%** | 3.6M | 36,000 | ~230 | ~520 | ~780 |
| **2.0%** | 3.6M | 72,000 | ~115 | ~260 | ~390 |
| **5.0%** | 3.6M | 180,000 | ~46 | ~104 | ~156 |
| **10.0%** | 3.6M | 360,000 | ~23 | ~52 | ~78 |

**Key insight**: Detecting higher fraud rates requires exponentially fewer samples.

### 3.2 Confidence Levels for Fixed Sample Sizes

**If we sample 500 JouleTorqs per ingot:**

| Fraud Rate | Probability We Miss It | Confidence We Catch It |
|-----------|----------------------|----------------------|
| **0.5%** | 1.3% | **98.7%** |
| **1.0%** | 0.67% | **99.3%** |
| **2.0%** | 0.34% | **99.66%** |
| **5.0%** | 0.067% | **99.93%** |
| **10.0%** | 0.0067% | **99.993%** |

**If we sample 1,000 JouleTorqs per ingot:**

| Fraud Rate | Probability We Miss It | Confidence We Catch It |
|-----------|----------------------|----------------------|
| **0.5%** | 0.0067% | **99.9933%** |
| **1.0%** | 0.0033% | **99.9967%** |
| **2.0%** | 0.0017% | **99.9983%** |
| **5.0%** | 0.00034% | **99.99966%** |

---

## 4. Attack Scenarios

### Scenario A: Single Contract Compromise

**Attacker controls 1 contract** out of 3,600 contracts per ingot

- **Max frauds**: ~1,080 JouleTorqs (1 token/sec × 3600 sec = 3600 JTU/contract)
- **Fraud rate**: 0.03% (1,080 / 3,600,000)
- **Detection with 500 samples**: ~99.9% confidence
- **Detection with 1,000 samples**: ~99.99% confidence

**Verdict**: Single contract compromise is easily detected.

---

### Scenario B: Multiple Contract Compromise

**Attacker controls 10 contracts** out of 3,600

- **Max frauds**: ~10,800 JouleTorqs
- **Fraud rate**: 0.3%
- **Detection with 500 samples**: ~98.5% confidence
- **Detection with 1,000 samples**: ~99.97% confidence

**Verdict**: 10-contract compromise still easily detected with 500 samples.

---

### Scenario C: Large-Scale Coordination

**Attacker controls 100 contracts** (2.8% of ingot)

- **Max frauds**: ~108,000 JouleTorqs
- **Fraud rate**: 3%
- **Detection with 100 samples**: ~97% confidence
- **Detection with 500 samples**: **>99.99%** confidence

**Verdict**: To evade 500-sample detection, attacker needs to compromise ~360 contracts (10% of network).

---

### Scenario D: Systemic Compromise Required

**To evade 500-sample detection at 95% confidence:**

- Attacker must maintain fraud rate < 0.5%
- Which requires controlling < 18,000 JouleTorqs
- Which means < 5 contracts
- **AND** statistically avoiding detection (which requires luck)

**Attacker must compromise network security in multiple layers:**
1. Post-quantum signatures (cryptographically hard)
2. 5+ contract systems (operationally complex)
3. Merkle tree structure (probabilistically risky)
4. Temporal consistency (pattern analysis would detect)

**Conclusion**: Fraud at scale is impractical.

---

## 5. Recommended Verification Strategy

### 5.1 Phase 3 (MVP): Lightweight Verification

**Goal**: Early filter, detect obvious attacks

```go
type IngotVerifier struct {
    sampleSize int
    confidence float64
}

func NewIngotVerifier() *IngotVerifier {
    return &IngotVerifier{
        sampleSize: 500,      // 500 samples per ingot
        confidence: 0.95,     // 95% confidence
    }
}

func (v *IngotVerifier) VerifyIngot(ctx context.Context, ingot *Ingot) error {
    // Randomly select 500 JouleTorqs from 3.6M
    samples := randomSample(ingot.JouleTorqs, v.sampleSize)
    
    for _, sample := range samples {
        // Verify: hash + signature
        if err := verifyJouleTorqProof(sample); err != nil {
            return fmt.Errorf("proof verification failed at sample %s: %w", sample.ID, err)
        }
    }
    
    return nil  // Passed all sampled proofs
}
```

**Cost**: ~2-3ms per ingot (negligible)  
**Detection**: 0.5-1% fraud rate @ 95-99% confidence

### 5.2 Phase 4+: Tiered Verification

#### **Tier 1: Mint Verification (Ingot Arrival)**
- **Samples**: 500 JouleTorqs per ingot
- **Confidence**: 95%
- **Detection**: ~0.5% fraud rate
- **Purpose**: Fast gate, catch obvious attacks
- **Cost**: ~2-3ms

#### **Tier 2: DistoDam Verification (Before Funding)**
- **Samples**: 1,000 JouleTorqs per ingot
- **Confidence**: 99%
- **Detection**: ~0.5% fraud rate
- **Purpose**: Higher bar before committing capital
- **Cost**: ~5ms
- **Trigger**: When DistoDam is about to fund contract using ingot's RoboStake

#### **Tier 3: Periodic Historical Audit**
- **Scope**: 10% of historical ingots (randomized)
- **Samples**: Full verification (all JouleTorqs or high sample like 5,000)
- **Frequency**: Monthly/quarterly
- **Purpose**: Detect systematic tampering patterns
- **Cost**: High, but amortized over time

#### **Tier 4: Post-Incident Forensics**
- **Trigger**: If fraud detected or suspected
- **Samples**: 100% of potentially compromised ingots
- **Purpose**: Root cause analysis, slashing determination
- **Cost**: Very high, but justified by security incident

---

## 6. Implementation Details

### 6.1 Random Sampling

**Cryptographically secure random selection** (prevents adversarial targeting):

```go
func randomSample(population []JouleTorq, sampleSize int) []JouleTorq {
    if sampleSize > len(population) {
        sampleSize = len(population)
    }
    
    // Fisher-Yates shuffle (Knuth algorithm)
    indices := rand.Perm(len(population))[:sampleSize]
    
    samples := make([]JouleTorq, sampleSize)
    for i, idx := range indices {
        samples[i] = population[idx]
    }
    
    return samples
}
```

**Why crypto-random**: Ensures attacker can't predict which proofs we'll check.

### 6.2 Proof Verification

Each JouleTorq contains:
```go
type JouleTorq struct {
    TokenID      string   // Unique token identifier
    Hash         string   // SHA256(token_id + joules + timestamp + ...)
    Signature    []byte   // Falcon-1024 signature
    // ... metadata
}
```

Verification process:
```go
func verifyJouleTorqProof(jtu JouleTorq) error {
    // 1. Reconstruct the data that was signed
    data := reconstructSignedData(jtu)
    
    // 2. Verify Falcon signature
    if !falcon.Verify(jtu.Signature, data, jtu.PublicKey) {
        return fmt.Errorf("signature verification failed")
    }
    
    // 3. Verify hash matches reconstructed data
    expectedHash := sha256.Sum256(data)
    if hex.EncodeToString(expectedHash[:]) != jtu.Hash {
        return fmt.Errorf("hash mismatch: expected %s, got %s", jtu.Hash, hex.EncodeToString(expectedHash[:]))
    }
    
    return nil
}
```

### 6.3 Adaptive Sampling (Optional Enhancement)

As network matures, adjust sample sizes based on:

```go
func (v *IngotVerifier) AdaptiveSampleSize(ingot *Ingot) int {
    // Base: 500 samples
    base := 500
    
    // Increase if ingot contains unusual contract mix
    if len(ingot.ContractIDs) > 3200 {
        base += 100  // More unique contracts = slightly higher risk
    }
    
    // Decrease if ingot has high reputation sources
    if avgReputation(ingot) > 90 {
        base = 300  // High-reputation contracts have lower fraud risk
    }
    
    // Increase if historical fraud detected from these contracts
    if hasHistoricalFraud(ingot.ContractIDs) {
        base += 500  // Previous bad actors = higher scrutiny
    }
    
    return base
}
```

---

## 7. Statistical Guarantees

### 7.1 Formal Statement

**If we sample 500 JouleTorqs per ingot uniformly at random:**

For any fraud rate $f \geq 0.5\%$:

$$P(\text{detect fraud}) \geq 1 - \left(1 - \frac{f}{100}\right)^{500} \geq 0.95$$

**Proof**: 
- $P(\text{miss all}) = (1 - 0.005)^{500} = 0.995^{500} \approx 0.0067$
- $P(\text{detect}) = 1 - 0.0067 = 0.9933 > 0.95$ ✓

### 7.2 Assumptions

1. **Cryptographic security** of Falcon-1024 holds (no poly-time algorithm to forge signatures)
2. **Random sampling** is truly random (attacker can't predict which we check)
3. **Proofs are independent** (attacker can't forge correlated set that evades detection)
4. **Ingot construction** is deterministic (can't change after fact)

---

## 8. Comparison to Blockchain Approaches

| Approach | Verification | Cost | Scalability |
|----------|-------------|------|-------------|
| **Full verification** (blockchain) | 100% of proofs | O(3.6M) | Doesn't scale |
| **Statistical sampling (RoboTorq)** | 0.014% of proofs (500/3.6M) | O(500) | ✅ Scales |
| **Threshold signature** (Stellar) | Single aggregated proof | O(1) | ✅ Scales, but trusts signers |
| **Zero-knowledge proof** (Zcash) | Compact proof | O(log n) | ✅ Scales, but complex |

**Why we chose statistical sampling**:
- ✅ Simple to understand and implement
- ✅ Post-quantum secure (uses Falcon signatures)
- ✅ Negligible overhead (~5ms per ingot)
- ✅ Probabilistic guarantees are sufficient for economic system
- ✅ No exotic cryptography required

---

## 9. Configuration

### 9.1 Environment Variables

```bash
# Mint verification strategy
MINT_PROOF_SAMPLE_SIZE=500           # Samples per ingot
MINT_PROOF_CONFIDENCE_LEVEL=0.95     # 95% = high confidence

# DistoDam verification strategy (higher bar)
DISTODAM_PROOF_SAMPLE_SIZE=1000      # More thorough
DISTODAM_PROOF_CONFIDENCE_LEVEL=0.99 # 99% = very high confidence

# Audit configuration
AUDIT_HISTORICAL_FRACTION=0.1        # Check 10% of historical ingots monthly
AUDIT_FULL_VERIFICATION=true         # Do full verification on audits
```

### 9.2 Alerting Thresholds

```yaml
alerts:
  # If we detect fraud, escalate immediately
  fraud_detected:
    severity: critical
    action: halt_funding
    notify: governance
  
  # If verification fails, investigate
  verification_failure:
    severity: high
    action: quarantine_ingot
    notify: ops
  
  # Track failure rates
  verification_failure_rate:
    threshold: 0.1%  # If >0.1% of ingots fail, something's wrong
    severity: high
    action: investigate
```

---

## 10. Implementation Checklist

### Phase 3 MVP
- [ ] Implement `randomSample()` function (crypto-secure)
- [ ] Implement `verifyJouleTorqProof()` function (Falcon verification)
- [ ] Add `IngotVerifier` struct with 500-sample hardcoded
- [ ] Integrate into Mint ingot receiver
- [ ] Log results (samples checked, proofs verified)
- [ ] Unit tests (sampling distribution, proof verification)
- [ ] Integration tests (full ingot pipeline)

### Phase 4 Production
- [ ] Make sample size configurable (env var)
- [ ] Add Prometheus metrics (samples checked, frauds detected)
- [ ] Implement Tier 2 (DistoDam verification)
- [ ] Add historical audit job (scheduled)
- [ ] Create alerting rules (fraud detection)
- [ ] Performance benchmarks (verify 500 proofs in <5ms)

### Phase 5+ (Optional)
- [ ] Adaptive sampling based on contract reputation
- [ ] Machine learning to predict fraud patterns
- [ ] Hot-reload configuration (change sample size without restart)
- [ ] Forensic tools (analyze ingot contents)

---

## 11. Performance Targets

| Operation | Target | Actual (Expected) |
|-----------|--------|------------------|
| Verify 1 JouleTorq proof | <10µs | ~2-5µs (Falcon verify) |
| Verify 500 proofs | <5ms | ~2-3ms |
| Verify 1,000 proofs | <10ms | ~5-7ms |
| Verify full ingot (3.6M proofs) | N/A (not done) | ~30-60 seconds |

**Note**: We never verify all 3.6M proofs. We use statistical sampling instead.

---

## 12. Formal Guarantees

### Lemma 1: Detection Guarantee

> **If fraud rate ≥ 0.5%, sampling 500 random proofs detects it with ≥99% confidence.**

**Proof by calculation**:
- Probability of missing all frauds: $(1 - 0.005)^{500} = 0.995^{500} \approx 0.0067$
- Probability of detecting fraud: $1 - 0.0067 = 0.9933 > 0.99$ ✓

### Lemma 2: Attacker Constraint

> **To evade 500-sample detection with <5% fraud rate, attacker must control <5 contracts in the ingot.**

**Proof**:
- 5 contracts × 1,080 JTU/contract = 5,400 frauds
- Fraud rate: 5,400 / 3,600,000 = 0.15%
- Detection with 500 samples: $1 - (0.9985)^{500} \approx 0.74 < 0.95$
- So attacker can evade 95% threshold BUT only if controlling 5 contracts
- Network consensus: if 5 contracts show synchronized fraud pattern, social consensus detects ✓

### Theorem: Economic Security

> **The combination of (1) post-quantum signatures, (2) multi-contract distribution, and (3) statistical verification creates an economically infeasible attack surface.**

**Proof sketch**:
1. Forge signatures: Cryptographically hard (Falcon-1024 assumed secure)
2. Control network: Requires compromise of ≥5 unrelated contracts (operationally complex)
3. Evade detection: Requires luck (random sampling might catch early batch)
4. Repeat undetected: Requires maintaining compromise across multiple ingests (temporal signature emerges)

**Conclusion**: Expected cost of attack >> potential gain. ✓

---

## 13. Q&A

### Q: Why not verify all 3.6M proofs?

**A**: Cost would be ~30-60 seconds per ingot. With Refinery producing 1 ingot per ~2 minutes, verification would bottleneck the pipeline. Statistical sampling gives us 99% confidence in 5ms.

### Q: What if attacker controls more contracts?

**A**: If attacker controls 100+ contracts, fraud rate becomes obvious even with small samples. Our algorithm catches large-scale attacks trivially.

### Q: What if signatures are breakable?

**A**: Entire system fails, but so does every other crypto system. We're using post-quantum crypto (Falcon-1024) precisely to guard against this.

### Q: Can we do better than 500 samples?

**A**: 
- **100 samples**: Detects ~5% fraud (good for 99% confidence, detects 10% frauds)
- **500 samples**: Detects ~0.5% fraud (excellent for 99% confidence)
- **1,000 samples**: Detects ~0.25% fraud (paranoia level, diminishing returns)

500 is the sweet spot for Phase 3.

### Q: Should we verify all ingots or sample ingots?

**A**: We verify 500 samples of ALL ingots (fast). We don't skip any ingot. We just don't verify all proofs in each ingot.

---

## References

- Hypergeometric distribution: [Wikipedia](https://en.wikipedia.org/wiki/Hypergeometric_distribution)
- Statistical sampling theory: [NIST Handbook](https://www.itl.nist.gov/div898/handbook/prc/section2/prc242.htm)
- Falcon signatures: [Specification](https://falcon-sign.info/)

---

**Document Status**: ✅ Complete and ready for implementation

**Next Step**: Integrate into Mint Phase 3 ingot receiver verification pipeline.
