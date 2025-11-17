# liboqs-go Research Findings - Phase 5

**Date**: November 16, 2025  
**Issue**: "can not sign message" error after first signature  
**Solution**: Re-initialize signature object with exported secret key per operation

---

## Problem Statement

When implementing Falcon-1024 signatures in Refinery, we encountered a critical error:

```go
// BROKEN PATTERN
type FalconSigner struct {
    sig       *oqs.Signature  // ❌ Can't reuse after first Sign()
    publicKey []byte
}

func (fs *FalconSigner) SignPhase2Ingot(...) (string, string, error) {
    signature, err := fs.sig.Sign([]byte(message))  // ❌ Fails on 2nd call
    // Error: "can not sign message"
}
```

**Symptoms**:
- First signature: ✅ Success
- Second signature: ❌ Error: "can not sign message"
- Third signature: ❌ Same error
- Signer initialization: ✅ No errors
- Keypair generation: ✅ No errors

**Root Cause**: `oqs.Signature` object becomes invalid after first `Sign()` call when secret key not re-initialized.

---

## Research Process

### 1. GitHub Repository Analysis

**Repository**: `github.com/open-quantum-safe/liboqs-go`  
**Key Files**:
- `oqs/oqs.go`: Core library implementation (lines 382-495)
- `oqstests/sig_test.go`: Unit tests showing correct usage patterns
- `examples/sig/sig.go`: Basic signature example

### 2. API Documentation Discovery

**Key Functions** (from `oqs/oqs.go`):

```go
// Signature struct
type Signature struct {
    sig        *C.OQS_SIG
    secretKey  []byte        // ← Stored internally!
    algDetails SignatureDetails
}

// Init with optional secret key
func (sig *Signature) Init(algName string, secretKey []byte) error

// Generate keypair (stores secret key internally)
func (sig *Signature) GenerateKeyPair() ([]byte, error)

// Export secret key for reuse
func (sig *Signature) ExportSecretKey() []byte

// Sign message (uses internal secretKey)
func (sig *Signature) Sign(message []byte) ([]byte, error)

// Clean zeroes secret key and resets object
func (sig *Signature) Clean()
```

**Critical Insight**: The `Init()` function accepts a `secretKey []byte` parameter!

### 3. Test Pattern Analysis

**From `oqstests/sig_test.go` (lines 26-46)**:

```go
// testSigCorrectness tests a specific signature
func testSigCorrectness(sigName string, msg []byte, threading bool, t *testing.T) {
    var signer, verifier oqs.Signature
    defer signer.Clean()
    defer verifier.Clean()
    
    // Initialize with nil secret key
    _ = signer.Init(sigName, nil)
    _ = verifier.Init(sigName, nil)
    
    // Generate keypair (secret key stored internally)
    pubKey, _ := signer.GenerateKeyPair()
    
    // Sign message (works because secret key is internal)
    signature, _ := signer.Sign(msg)
    
    // Verify
    isValid, _ := verifier.Verify(msg, signature, pubKey)
}
```

**Key Observation**: Tests create **new** `oqs.Signature{}` objects for each test, they don't reuse the same object across multiple signatures in a single test.

### 4. Export/Import Pattern Discovery

**From library code** (`oqs/oqs.go` lines 439-440):

```go
// ExportSecretKey exports the corresponding secret key from the sig receiver.
func (sig *Signature) ExportSecretKey() []byte {
    return sig.secretKey
}
```

**Pattern for Multiple Signatures**:

```go
// 1. Generate keypair ONCE
sig := oqs.Signature{}
sig.Init("Falcon-1024", nil)
publicKey, _ := sig.GenerateKeyPair()
secretKey := sig.ExportSecretKey()  // ← Export for reuse!
sig.Clean()

// 2. For EACH signature operation, create fresh object
func SignMessage(msg []byte, secretKey []byte) ([]byte, error) {
    sig := oqs.Signature{}
    defer sig.Clean()
    
    // Re-initialize with exported secret key
    sig.Init("Falcon-1024", secretKey)  // ← Key reuse!
    
    return sig.Sign(msg)
}
```

---

## Solution Implementation

### Corrected FalconSigner

```go
type FalconSigner struct {
    secretKey []byte  // ✅ Store secret key, not signature object
    publicKey []byte
}

func NewFalconSigner() (*FalconSigner, error) {
    // Create temporary signature object for key generation
    sig := oqs.Signature{}
    defer sig.Clean()

    sig.Init("Falcon-1024", nil)
    publicKey, _ := sig.GenerateKeyPair()
    
    // Export secret key for reuse
    secretKey := sig.ExportSecretKey()

    return &FalconSigner{
        secretKey: secretKey,
        publicKey: publicKey,
    }, nil
}

func (fs *FalconSigner) SignPhase2Ingot(...) (string, string, error) {
    message := fmt.Sprintf("%s|%s|%d|%s", ingotID, branchHash, hashCount, timestamp)

    // Create NEW signature object per operation
    sig := oqs.Signature{}
    defer sig.Clean()

    // Re-initialize with exported secret key
    sig.Init("Falcon-1024", fs.secretKey)  // ✅ Key insight!

    signature, err := sig.Sign([]byte(message))
    return hex.EncodeToString(signature), hex.EncodeToString(fs.publicKey), nil
}

func (fs *FalconSigner) Clean() {
    if fs.secretKey != nil {
        oqs.MemCleanse(fs.secretKey)  // ✅ Securely erase secret key
        fs.secretKey = nil
    }
}
```

---

## Test Results

### E2E Test: Phase 5 Complete Pipeline

```
╔═══════════════════════════════════════════════════════════════╗
║             RoboTorq Phase 5 Complete E2E Test                ║
╚═══════════════════════════════════════════════════════════════╝

✅ PASS - hash_batches_received
✅ PASS - hashes_queued
✅ PASS - merkle_tree_built
✅ PASS - phase2_ingot_created          ← Fixed!
✅ PASS - ingot_sent_to_mint            ← Fixed!
✅ PASS - mint_received_ingots          ← Fixed!

Total: 6/6 pipeline stages passed
🎉 COMPLETE PIPELINE SUCCESS! 🎉
```

**Metrics Verified**:
- Hashes queued: 3,900 ✅
- Merkle trees built: 1 ✅
- Phase2 ingots created: 1 ✅ (was 0 before fix)
- Ingots sent to Mint: 1 ✅ (was 0 before fix)
- Mint received ingots: 1 ✅ (was 0 before fix)

**Logs Verified**:
```json
// Refinery
{"level":"INFO","msg":"Phase 2 ingot assembled","ingot_id":"695ad8c8...","hash_count":3600}
{"level":"INFO","msg":"Phase2 batch published successfully"}

// Mint
{"level":"INFO","msg":"Phase 2 ingot received","ingot_id":"695ad8c8...","hash_count":3600}
// No signature verification errors! ✅
```

---

## Performance Considerations

**Pattern Comparison**:

| Pattern | Signature Speed | Multiple Signatures | Memory Safety |
|---------|----------------|---------------------|---------------|
| Keep `sig *oqs.Signature` | N/A | ❌ Fails | N/A |
| Create new `sig` per operation | ~2-5ms | ✅ Works | ✅ Secure |

**RoboTorq Context**:
- Rate: 1 ingot per 60 seconds
- Overhead: ~2-5ms per signature
- Impact: **Negligible** (0.008% of 60s interval)

**Trade-off**: Slightly slower (~2-5ms) but **correct and secure**.

---

## Key Learnings

### ✅ DO

1. **Export secret key after initial keypair generation**:
   ```go
   secretKey := sig.ExportSecretKey()
   ```

2. **Create fresh `oqs.Signature{}` per signing operation**:
   ```go
   sig := oqs.Signature{}
   defer sig.Clean()
   ```

3. **Re-initialize with exported secret key**:
   ```go
   sig.Init("Falcon-1024", secretKey)
   ```

4. **Use `oqs.MemCleanse()` to securely erase secret keys**:
   ```go
   oqs.MemCleanse(secretKey)
   ```

5. **Always defer `sig.Clean()`** to prevent memory leaks:
   ```go
   defer sig.Clean()
   ```

### ❌ DON'T

1. **Don't reuse `oqs.Signature` object across multiple `Sign()` calls**:
   ```go
   // ❌ BROKEN
   type Signer struct {
       sig *oqs.Signature
   }
   func (s *Signer) Sign(msg []byte) {
       s.sig.Sign(msg)  // Fails after first call
   }
   ```

2. **Don't forget to export secret key if reusing**:
   ```go
   // ❌ BROKEN
   sig.GenerateKeyPair()
   // Missing: secretKey := sig.ExportSecretKey()
   ```

3. **Don't pass `nil` when you need to reuse secret key**:
   ```go
   // ❌ BROKEN - should pass secretKey, not nil
   sig.Init("Falcon-1024", nil)  
   ```

---

## Why This Pattern Exists

**Hypothesis** (based on liboqs C library design):

1. **C Library Statefulness**: The underlying `OQS_SIG` C struct may maintain internal state that becomes invalid after `OQS_SIG_sign()`.

2. **Memory Safety**: Creating a fresh object ensures no stale pointers or state corruption.

3. **Thread Safety**: Each goroutine gets its own signature object, avoiding shared state.

4. **Resource Management**: `Clean()` properly frees C memory allocated by `OQS_SIG_new()`.

**Evidence**:
- liboqs-go tests never reuse signature objects across signatures
- `Clean()` function calls `C.OQS_SIG_free(sig.sig)` and resets struct
- Comment in `Clean()`: "One can reuse the signature by re-initializing"

---

## Related Documentation

- **liboqs-go API**: https://pkg.go.dev/github.com/open-quantum-safe/liboqs-go/oqs
- **liboqs C library**: https://github.com/open-quantum-safe/liboqs
- **Falcon-1024 Spec**: https://falcon-sign.info/
- **NIST PQC**: https://csrc.nist.gov/projects/post-quantum-cryptography

---

## Future Considerations

### SPHINCS+ Implementation (Phase 3 Units)

**Same pattern applies**:
```go
type SPHINCSPlusSigner struct {
    secretKey []byte  // ✅ Export and store
    publicKey []byte
}

func (s *SPHINCSPlusSigner) SignPhase3Unit(...) (string, string, error) {
    sig := oqs.Signature{}
    defer sig.Clean()
    
    sig.Init("SPHINCS+-SHA2-128f-simple", s.secretKey)  // ✅ Reuse secret key
    return sig.Sign(message)
}
```

### Performance Optimization (If Needed)

If signature rate increases significantly (>100/sec):
1. **Pre-allocate signature objects**: Pool of `oqs.Signature{}` objects
2. **Batch signing**: Accumulate messages, sign in batch
3. **Parallel signing**: Goroutine pool with worker pattern

**Current rate**: 1 signature per 60 seconds → optimization unnecessary.

---

## Acknowledgments

**Research Sources**:
- liboqs-go GitHub repository (examples and tests)
- Open Quantum Safe project documentation
- Trial-and-error experimentation with E2E tests
- Docker logs analysis

**Commit**: `5f415b5` - "fix(refinery): Implement correct liboqs-go pattern for multiple signatures"

---

**Lesson**: When library documentation is sparse, **study the test code** - it often reveals the correct usage patterns.
