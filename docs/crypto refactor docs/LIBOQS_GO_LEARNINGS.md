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

**Root Cause Discovery** (deeper than initially thought):

The problem was MORE subtle than just "signature object becomes invalid." The actual issue:

1. `sig.Init(algName, secretKey)` **stores a REFERENCE** to the `secretKey` parameter (doesn't copy it)
2. `sig.Clean()` calls `MemCleanse(sig.secretKey)` which **zeroes the referenced slice**
3. When we passed `fs.secretKey` directly to `Init()`, `Clean()` would zero our original secret key
4. Second signature attempt uses zeroed memory → "can not sign message"

**Initial Fix** (incomplete):
```go
// This STILL had the bug!
secretKey := sig.ExportSecretKey()  // Returns reference, not copy
sig.Init("Falcon-1024", secretKey)  // Stores reference
sig.Clean()  // Zeroes secretKey!
```

**Complete Fix** (the copy is critical):
```go
// MUST copy before passing to Init()
secretKeyCopy := make([]byte, len(fs.secretKey))
copy(secretKeyCopy, fs.secretKey)
sig.Init("Falcon-1024", secretKeyCopy)  // Safe: Clean() only zeroes the copy
```

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
    
    // CRITICAL: Export AND COPY secret key
    exportedKey := sig.ExportSecretKey()
    secretKey := make([]byte, len(exportedKey))
    copy(secretKey, exportedKey)  // ✅ COPY before sig.Clean() zeroes it

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

    // CRITICAL: Make a COPY of the secret key before passing to Init()
    // Init() stores a REFERENCE to the parameter, not a copy
    // When Clean() calls MemCleanse(sig.secretKey), it zeroes the referenced slice
    secretKeyCopy := make([]byte, len(fs.secretKey))
    copy(secretKeyCopy, fs.secretKey)
    
    sig.Init("Falcon-1024", secretKeyCopy)  // ✅ Pass copy, not original!

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

### Unit Tests: Regression Test for Multiple Signatures

**Created**: `src/refinery/internal/crypto/falcon_test.go`

```go
func TestFalconSigner_SignPhase2Ingot_MultipleSignatures(t *testing.T) {
    signer, err := NewFalconSigner()
    require.NoError(t, err)
    defer signer.Clean()

    // Sign 3 different messages with same signer
    for i := 1; i <= 3; i++ {
        ingotID := fmt.Sprintf("test-ingot-%03d", i)
        sig, pubKey, err := signer.SignPhase2Ingot(ingotID, "hash", 3600, "2025-11-16T12:00:00Z")
        
        assert.NoError(t, err, "Signature %d should succeed", i)  // ✅ All pass now!
        assert.NotEmpty(t, sig)
        assert.NotEmpty(t, pubKey)
    }
}
```

**Results**:
```
=== RUN   TestFalconSigner_SignPhase2Ingot_MultipleSignatures
--- PASS: TestFalconSigner_SignPhase2Ingot_MultipleSignatures (0.09s)
```

### Integration Tests: Phase2IngotReceiver Signature Verification

**Created**: `src/mint/internal/mint/phase2_ingot_receiver_test.go`

Tests verify that Mint correctly accepts/rejects Falcon-1024 signatures:

```
=== RUN   TestPhase2IngotReceiver_ValidFalconSignature
--- PASS: TestPhase2IngotReceiver_ValidFalconSignature (0.35s)

=== RUN   TestPhase2IngotReceiver_InvalidFalconSignature
--- PASS: TestPhase2IngotReceiver_InvalidFalconSignature (0.34s)

=== RUN   TestPhase2IngotReceiver_TamperedMessage
--- PASS: TestPhase2IngotReceiver_TamperedMessage (0.33s)

=== RUN   TestPhase2IngotReceiver_MixedValidInvalid
--- PASS: TestPhase2IngotReceiver_MixedValidInvalid (0.54s)
```

**Key Validation**:
- ✅ Valid signatures accepted and queued
- ✅ Corrupted signatures rejected with error log
- ✅ Tampered messages detected (signature mismatch)
- ✅ Mixed batches: valid ingots queued, invalid rejected
- ✅ Metrics correctly tracked (success/failed counters)

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

1. **Export AND COPY secret key after initial keypair generation**:
   ```go
   exportedKey := sig.ExportSecretKey()
   secretKey := make([]byte, len(exportedKey))
   copy(secretKey, exportedKey)  // ✅ COPY is critical!
   ```

2. **Create fresh `oqs.Signature{}` per signing operation**:
   ```go
   sig := oqs.Signature{}
   defer sig.Clean()
   ```

3. **ALWAYS copy secret key before passing to Init()**:
   ```go
   secretKeyCopy := make([]byte, len(fs.secretKey))
   copy(secretKeyCopy, fs.secretKey)
   sig.Init("Falcon-1024", secretKeyCopy)  // ✅ Pass copy!
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

2. **Don't forget to COPY exported secret key**:
   ```go
   // ❌ BROKEN - ExportSecretKey() returns reference
   secretKey := sig.ExportSecretKey()  // sig.Clean() will zero this!
   ```

3. **Don't pass original secret key to Init()**:
   ```go
   // ❌ BROKEN - Init() stores reference, Clean() zeroes it
   sig.Init("Falcon-1024", fs.secretKey)  
   ```

4. **Don't forget to copy BEFORE Init()**:
   ```go
   // ❌ BROKEN - Clean() will zero fs.secretKey
   sig.Init("Falcon-1024", fs.secretKey)
   defer sig.Clean()
   ```

---

## Why This Pattern Exists

**Root Cause** (confirmed through testing and code analysis):

1. **`Init()` Stores Reference, Not Copy**:
   ```go
   func (sig *Signature) Init(algName string, secretKey []byte) error {
       sig.secretKey = secretKey  // ← Stores reference to parameter!
   }
   ```

2. **`Clean()` Zeroes Referenced Memory**:
   ```go
   func (sig *Signature) Clean() {
       if len(sig.secretKey) > 0 {
           MemCleanse(sig.secretKey)  // ← Zeroes the ORIGINAL slice!
       }
   }
   ```

3. **Why E2E Test Passed Initially**:
   - Only created ONE ingot per test run
   - Never exercised the multiple-signature code path
   - Bug only appeared when signing 2+ messages with same signer

4. **Why Unit Test Caught It**:
   - Explicitly tested multiple signatures in loop
   - Regression test for this exact scenario
   - Failed on second signature until fix applied

**Memory Management Philosophy**:
- liboqs-go expects you to manage secret key lifecycle
- `Init()` doesn't copy to avoid unnecessary allocations
- YOU must copy if you need to preserve the key across `Clean()` calls

---

## Related Documentation

- **liboqs-go API**: https://pkg.go.dev/github.com/open-quantum-safe/liboqs-go/oqs
- **liboqs C library**: https://github.com/open-quantum-safe/liboqs
- **Falcon-1024 Spec**: https://falcon-sign.info/
- **NIST PQC**: https://csrc.nist.gov/projects/post-quantum-cryptography

---

## Future Considerations

### SPHINCS+ Implementation (Phase 3 Units)

**Same pattern applies** (CRITICAL: copy secret key before Init):
```go
type SPHINCSPlusSigner struct {
    secretKey []byte  // ✅ Store copy, not reference
    publicKey []byte
}

func NewSPHINCSPlusSigner() (*SPHINCSPlusSigner, error) {
    sig := oqs.Signature{}
    defer sig.Clean()
    
    sig.Init("SPHINCS+-SHA2-128f-simple", nil)
    publicKey, _ := sig.GenerateKeyPair()
    
    // CRITICAL: COPY exported key
    exportedKey := sig.ExportSecretKey()
    secretKey := make([]byte, len(exportedKey))
    copy(secretKey, exportedKey)
    
    return &SPHINCSPlusSigner{
        secretKey: secretKey,
        publicKey: publicKey,
    }, nil
}

func (s *SPHINCSPlusSigner) SignPhase3Unit(...) (string, string, error) {
    sig := oqs.Signature{}
    defer sig.Clean()
    
    // CRITICAL: COPY secret key before Init()
    secretKeyCopy := make([]byte, len(s.secretKey))
    copy(secretKeyCopy, s.secretKey)
    
    sig.Init("SPHINCS+-SHA2-128f-simple", secretKeyCopy)  // ✅ Pass copy
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
- **Unit test debugging** - regression test revealed the Init() reference issue

**Key Commits**:
- `5f415b5` - "fix(refinery): Implement correct liboqs-go pattern for multiple signatures"
- `0956181` - "docs: Add liboqs-go research findings and best practices"

**Tests Created**:
- `src/refinery/internal/crypto/falcon_test.go` - 10 unit tests (all passing)
- `src/mint/internal/crypto/falcon_test.go` - 6 unit tests (all passing)
- `src/mint/internal/mint/phase2_ingot_receiver_test.go` - 6 integration tests (all passing)

---

**Lesson**: When library documentation is sparse, **study the test code** - it often reveals the correct usage patterns. And when tests pass but production fails, **write regression tests that match production's actual usage pattern** (multiple operations with same object).
