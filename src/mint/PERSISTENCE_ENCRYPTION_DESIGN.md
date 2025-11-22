# Mint Persistence Layer: Encryption & Key Management Design

**Status**: Architecture Design (NOT YET IMPLEMENTED)  
**Date**: November 20, 2025  
**Author**: RoboTorq Development Team  
**Constraint**: Post-quantum ready by design

---

## Executive Summary

The Mint persistence layer will upgrade from JSON file storage to **encrypted SQLite** with:
- **Master key sealing** via Kyber-1024 KEM (post-quantum resistant)
- **Data encryption** via AES-256-GCM (current standard, migration path documented)
- **Environment-based key injection** (no human sees raw keys)
- **Crash recovery** with atomic transactions and replay protection

This design balances **security**, **operability**, and **post-quantum readiness** without over-engineering.

---

## Current State vs. Future State

### Current (Phase 3 - Deployed)
```
┌──────────────┐
│ IngotStore   │  JSON files on disk (unencrypted)
├──────────────┤
│ ProofStore   │  JSON files on disk (unencrypted)
├──────────────┤
│ IngotHashEntry│ Plain text: BranchHash, RoboStakeTotal
└──────────────┘

Risks:
- Database file stolen → all proofs readable
- No atomic transactions → crash corruption possible
- No key rotation → secrets locked in codebase
```

### Future State (Phase 4 - Planned)
```
┌───────────────────────────────────────────────────────────┐
│ GitHub Secrets (encrypted at rest)                        │
│ ├─ MINT_MASTER_KEY (32 bytes, random, rotated monthly)   │
│ └─ MINT_KYBER_PRIVATE_KEY (Kyber-1024 private key)       │
└──────────────┬────────────────────────────────────────────┘
               │ (injected at container start via CI/CD)
┌──────────────▼────────────────────────────────────────────┐
│ Container Environment (temporary)                         │
│ ├─ Both secrets in memory (volatile)                     │
│ └─ Never logged, never written to disk                   │
└──────────────┬────────────────────────────────────────────┘
               │ (read by Mint at boot)
┌──────────────▼────────────────────────────────────────────┐
│ Mint Service Memory                                       │
│ ├─ Load master key from MINT_MASTER_KEY env var         │
│ ├─ Verify integrity (MAC)                               │
│ └─ Ready to decrypt database                            │
└──────────────┬────────────────────────────────────────────┘
               │ (AES-256-GCM encryption/decryption)
┌──────────────▼────────────────────────────────────────────┐
│ SQLite Database (encrypted at rest)                       │
│ ├─ in_flight_ingots table (crash recovery)              │
│ ├─ phase3_proofs table (permanent audit trail)          │
│ ├─ persistence_metadata table (version, rotation info)   │
│ └─ All fields encrypted with AES-256-GCM                │
└───────────────────────────────────────────────────────────┘
```

---

## Cryptographic Design

### 1. Master Key Management (Kyber-1024 KEM)

**Purpose**: Protect the master key itself (defense-in-depth)

**Flow**:
```
CI/CD Pipeline (GitHub Actions):
├─ Generate random 256-bit master key (32 bytes)
├─ Generate Kyber-1024 ephemeral keypair
├─ Encapsulate master key with Kyber public key
├─ Create sealed secret: <encapsulation || ciphertext>
├─ Store in GitHub Secrets as MINT_MASTER_KEY_SEALED
├─ Store Kyber private key in MINT_KYBER_PRIVATE_KEY secret
└─ → Nobody sees raw master key in plaintext

Mint Container Startup:
├─ Read MINT_KYBER_PRIVATE_KEY from env (GitHub injected)
├─ Read MINT_MASTER_KEY_SEALED from env
├─ Decapsulate sealed master key using Kyber private key
├─ Verify MAC of decapsulated key (was it tampered?)
├─ Load into memory for AES operations
└─ Kyber private key stays in memory (never written to disk)

Restart Behavior:
├─ Same environment variables → same master key
├─ All historical proofs readable (key matches)
├─ Crash recovery works: SQLite transactions replay correctly
```

**Why Kyber-1024?**
- ✅ NIST-standardized (ML-KEM, 2022)
- ✅ Post-quantum resistant (lattice-based)
- ✅ Key encapsulation (not just signing)
- ✅ Already available in liboqs (which we use for Falcon)
- ✅ Protects key material from both classical AND quantum attacks

**Threat Coverage**:
| Threat | Coverage |
|--------|----------|
| GitHub Secrets leaked (human error) | N/A (access control problem, not crypto) |
| Kyber private key compromised | Attacker can decrypt master key, but only current version |
| Master key in transit | Protected by Kyber KEM, not readable without private key |
| Quantum attacker with future computer | Lattice problem (Kyber) remains hard even for quantum |

---

### 2. Data Encryption (AES-256-GCM)

**Purpose**: Encrypt all data at rest in SQLite

**Flow**:
```
Write Operation:
├─ Generate random 96-bit nonce (12 bytes)
├─ Call AES-256-GCM encrypt:
│  ├─ Plaintext: IngotHashEntry JSON
│  ├─ Key: master_key (32 bytes)
│  ├─ Nonce: random (12 bytes)
│  ├─ AAD: unit_id (authenticated but not encrypted)
│  └─ Output: ciphertext || auth_tag (16 bytes)
├─ Store in SQLite:
│  ├─ unit_id (plaintext, indexed for queries)
│  ├─ nonce (plaintext, needed for decryption)
│  ├─ ciphertext (encrypted data)
│  └─ auth_tag (verify integrity on read)
└─ → Data is encrypted, authenticated, tamper-evident

Read Operation:
├─ Fetch row from SQLite (nonce, ciphertext, auth_tag)
├─ Call AES-256-GCM decrypt:
│  ├─ Ciphertext: from database
│  ├─ Key: master_key
│  ├─ Nonce: from database
│  ├─ AAD: unit_id
│  └─ Auth tag: verify (if wrong, decryption fails)
├─ On auth failure: Reject (data tampered or wrong key)
└─ → Return decrypted IngotHashEntry

Restart After Crash:
├─ Same master key loaded
├─ All rows decrypt successfully (nonce + key matches)
├─ Transactions replay from SQLite journal
└─ Complete recovery guaranteed
```

**Why AES-256-GCM?**
- ✅ Authenticated encryption (AEAD mode)
- ✅ 256-bit strength (2^128 effective against Grover's algorithm)
- ✅ Hardware-accelerated (AES-NI on modern CPUs)
- ✅ Standard library support (Go crypto/aes)
- ✅ NIST approved (SP 800-38D)

**Post-Quantum Status**:
- ⚠️ **NOT** quantum-resistant (Grover's algorithm reduces 2^256 → 2^128)
- ✅ **Sufficient** for 20+ years per NIST guidance (2^128 is still strong)
- 🔄 **Migration path**: Upgrade to PQ-AEAD when NIST standardizes (expected 2026)

---

### 3. Schema Design

**Table 1: in_flight_ingots** (Crash Recovery)
```sql
CREATE TABLE in_flight_ingots (
  batch_id TEXT PRIMARY KEY,
  ingot_entries BLOB NOT NULL,        -- JSON (encrypted): [IngotHashEntry...]
  nonce BLOB NOT NULL,                 -- 12 bytes (random, for AES-GCM)
  auth_tag BLOB NOT NULL,              -- 16 bytes (MAC of encrypted data)
  created_at INTEGER NOT NULL,         -- Unix timestamp
  updated_at INTEGER NOT NULL,         -- Unix timestamp
  format_version INTEGER DEFAULT 1     -- Schema version for migrations
);

CREATE INDEX idx_ingots_created ON in_flight_ingots(created_at);
CREATE INDEX idx_ingots_updated ON in_flight_ingots(updated_at);
```

**Purpose**: 
- Recover ingot batches if Mint crashes mid-assembly
- Example: Mint received 300 units, accumulated 2500, crash before reaching 3600
- On restart: Reload batch, resume accumulation

**Recovery Flow**:
```
Crash occurs at: IngotHashQueue.SaveIngotBatch("batch-001", entries)
Recovered on restart: ReadAllBatches() scans all rows
Ingot assembly resumes: IngotHashQueue.GetIngotHashes() returns recovered batch
```

**Table 2: phase3_proofs** (Permanent Audit Trail)
```sql
CREATE TABLE phase3_proofs (
  unit_id TEXT PRIMARY KEY,            -- Unique Phase3RoboTorqUnit ID
  merkle_root TEXT NOT NULL UNIQUE,    -- Index for lookups
  proof_data BLOB NOT NULL,            -- JSON encrypted: merkle tree, signatures
  nonce BLOB NOT NULL,                 -- 12 bytes (random, for AES-GCM)
  auth_tag BLOB NOT NULL,              -- 16 bytes (MAC of encrypted data)
  robo_stake_total REAL NOT NULL,      -- Plaintext metadata (not encrypted)
  tree_height INTEGER NOT NULL,        -- Plaintext metadata
  minted_at INTEGER NOT NULL,          -- Plaintext: when minted
  persisted_at INTEGER NOT NULL,       -- Plaintext: when written to disk
  format_version INTEGER DEFAULT 1     -- Schema version
);

CREATE INDEX idx_proofs_merkle ON phase3_proofs(merkle_root);
CREATE INDEX idx_proofs_persisted ON phase3_proofs(persisted_at);
CREATE INDEX idx_proofs_minted ON phase3_proofs(minted_at);
```

**Purpose**:
- Permanent audit trail of all minted RoboTorq units
- CSV export for compliance/audits
- Merkle tree lookups for proof verification

**Plaintext Metadata** (unencrypted):
- `unit_id`, `merkle_root` → indexed for queries (query without decryption)
- `robo_stake_total`, `tree_height` → used in aggregates
- `minted_at`, `persisted_at` → timestamps for cleanup/analytics

**Encrypted Data** (in `proof_data` BLOB):
- Complete Phase3RoboTorqUnit (contracts, digger ID, signatures)
- Merkle tree structure
- Branch hashes

**Table 3: persistence_metadata** (Version & Rotation Tracking)
```sql
CREATE TABLE persistence_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

-- Records:
-- key='format_version', value='1'                       -- Schema version
-- key='key_rotation_count', value='0'                   -- How many rotations
-- key='last_key_rotation', value='1732000000'           -- Timestamp
-- key='encryption_algorithm', value='AES-256-GCM'       -- Current cipher
-- key='master_key_mac', value='<sha256(key)>'           -- Verify key didn't change
```

**Purpose**:
- Track encryption algorithm version (for migrations)
- Detect unexpected key changes
- Audit trail of key rotations

---

## Key Rotation Strategy

### Scenario: Monthly Master Key Rotation

**Old State**:
```
Old master key: K1 (32 bytes)
Encrypted proofs: 10,000 Phase3RoboTorqUnits
```

**Rotation Process**:
```
Step 1: CI/CD generates new key
├─ Generate new 32-byte master key K2
├─ Seal with Kyber → store in MINT_MASTER_KEY_SEALED_NEW
└─ Keep MINT_MASTER_KEY_SEALED_OLD for transition

Step 2: Mint detects key change (startup)
├─ Try to decrypt existing proofs with K2
├─ Fails (was encrypted with K1)
├─ Log: "Key rotation detected, performing migration"
└─ (Can also be manual trigger)

Step 3: Re-encrypt all proofs
├─ Decrypt all proofs with K1 (old key)
├─ For each proof:
│  ├─ Generate new nonce N2
│  ├─ Encrypt with K2 (new key)
│  ├─ Update SQLite row with new ciphertext, nonce, auth_tag
│  └─ Log progress
├─ Verify: spot-check decrypt
└─ On success: mark migration complete

Step 4: Update metadata
├─ Set persistence_metadata['key_rotation_count'] = N+1
├─ Set persistence_metadata['last_key_rotation'] = now()
└─ Set persistence_metadata['master_key_mac'] = sha256(K2)

Step 5: Clean up old key
├─ Environment no longer has MINT_MASTER_KEY_SEALED_OLD
├─ K1 is not in memory anymore
└─ Next restart: only K2 available
```

**Time Complexity**: O(n) for n proofs (batch re-encryption)  
**Downtime**: Performed on startup (optional migration window)  
**Rollback**: Keep old key in GitHub Secrets, restore old container version

---

## Implementation Phases

### Phase 4A: SQLite Infrastructure (NOT YET IMPLEMENTED)
- [ ] Add `github.com/mattn/go-sqlite3` dependency
- [ ] Create `database.go` with connection pooling
- [ ] Implement schema creation (tables + indexes)
- [ ] Add connection health checks

**Estimated**: 4-6 hours  
**Risk**: Low (well-tested library)

### Phase 4B: Encryption Layer (NOT YET IMPLEMENTED)
- [ ] Add `liboqs-go/liboqs_test.go` (Kyber import)
- [ ] Create `crypto.go` with AES-256-GCM functions
- [ ] Implement `encryptValue()` and `decryptValue()` helpers
- [ ] Add MAC verification and nonce handling

**Estimated**: 6-8 hours  
**Risk**: Medium (crypto requires careful implementation)

### Phase 4C: Data Access Layer (NOT YET IMPLEMENTED)
- [ ] Replace current `ingot_store.go` with SQL version
- [ ] Replace current `proof_store.go` with SQL version
- [ ] Migrate all tests from JSON to database backend
- [ ] Verify crash recovery scenarios

**Estimated**: 8-12 hours  
**Risk**: Medium (complex state transitions)

### Phase 4D: Integration & Testing (NOT YET IMPLEMENTED)
- [ ] Integrate with IngotHashQueue
- [ ] Integrate with Phase3Assembler
- [ ] Add graceful shutdown + flush logic
- [ ] E2E test: crash recovery, key rotation

**Estimated**: 6-10 hours  
**Risk**: Medium (integration complexity)

**Total Estimated Effort**: 24-36 hours (3-4 days of focused work)

---

## Post-Quantum Encryption Roadmap

### Timeline

**2025 (Now)**: AES-256-GCM as primary  
- Documented as transitional
- 2^128 effective strength (sufficient for 20+ years)
- Migration path planned

**2026 (Expected)**: NIST standardizes PQ-AEAD  
- ML-KEM (Kyber) already standardized for key encapsulation
- Waiting on symmetric AEAD standard (likely liboqs will add)

**2026-2027**: Implement dual-encryption  
- Option A: Decrypt with AES, re-encrypt with PQ-AEAD
- Option B: Wrap AES-256-GCM output with PQ-AEAD layer
- Transparent to application (migration internal)

**2027-2028**: Migrate historical data  
- Batch re-encryption job (background process)
- Gradual conversion of all proofs to PQ-AEAD
- Keep AES decryption capability for audit trail

**2028+**: Deprecate AES-256-GCM  
- All new data encrypted with PQ-AEAD
- Old data still readable with legacy AES code

### Contingency Plan

If PQ-AEAD standardization is delayed:
```
├─ Option A: Use Kyber KEM + ChaCha20-Poly1305
│  ├─ Kyber = PQ-resistant key establishment
│  └─ ChaCha20 = symmetric cipher (not PQ-resistant, but unconventional)
├─ Option B: Wait, keep AES-256 longer
│  └─ 2^128 effective strength = safe until ~2045 (NIST guidance)
└─ Option C: Hybrid triple-encryption (paranoid)
    └─ AES-256 + Kyber KEM + homomorphic encryption (overkill)
```

---

## Environment Variables

**Production Deployment**:
```bash
# GitHub Secrets (injected by CI/CD)
MINT_MASTER_KEY="<32 random bytes, base64-encoded>"
MINT_KYBER_PRIVATE_KEY="<Kyber-1024 private key, base64-encoded>"

# Optional
MINT_KEY_ROTATION_MODE="on_startup" | "manual"  # When to rotate
MINT_ENCRYPTION_ALGORITHM="aes-256-gcm"        # Current algorithm
MINT_DB_PATH="/data/mint.db"                    # SQLite location
```

**Development**:
```bash
# Use fixed keys for reproducibility
MINT_MASTER_KEY="base64:dGVzdGtleXdpdGgzMmJ5dGVzbm93dGhlcmU="
MINT_KYBER_PRIVATE_KEY="base64:<dummy-key-for-testing>"
```

---

## Security Properties

### Threat Model & Mitigations

| Threat | Severity | Mitigation | Residual Risk |
|--------|----------|-----------|---|
| SQLite file stolen from disk | HIGH | AES-256-GCM encryption | None (2^128 effective strength) |
| GitHub Secrets compromised | CRITICAL | Access control, audit logging | Depends on GitHub's infrastructure |
| Master key in env logs | HIGH | Redact logs, no logging of secrets | Process discipline required |
| Quantum attacker (future) | MEDIUM | Kyber-1024 KEM for key, migrate to PQ-AEAD | Mitigated by 2027 |
| Nonce reuse in AES-GCM | CRITICAL | Random nonce per write, collision test | Cryptographically negligible (2^-96) |
| Crash during write | MEDIUM | SQLite transactions + journal | Handled by SQLite ACID |
| Key rotation botched | HIGH | Backup old key in GitHub, manual recovery | Runbook required |
| Container memory dumped | MEDIUM | Master key only in memory (volatile) | Can't prevent, but limits window |
| Tampered database | MEDIUM | AES-GCM auth_tag verification | Rejected on read |

### Security Assumptions

1. **GitHub Secrets are secure** (encrypted at rest, access controlled)
2. **Container runtime protects memory** (no rogue processes can read env vars)
3. **SQLite library is not compromised** (industry-standard, well-audited)
4. **Falcon/SPHINCS+ signatures are trusted** (Phase3 proofs are authentic)
5. **Nonce generation is cryptographically random** (Go's crypto/rand)

---

## Testing Strategy (Phase 4C)

### Unit Tests (Not yet written)
```go
// crypto_test.go
func TestAES256GCM_EncryptDecrypt() // Roundtrip
func TestAES256GCM_AuthenticationFailure() // Wrong key
func TestAES256GCM_NonceIsolation() // Different nonce = different ciphertext
func TestKyberKEM_Encapsulation() // Seal/unseal master key
func TestKeyRotation_Migration() // Decrypt old, encrypt new

// database_test.go
func TestPhase3ProofStore_Encrypted() // Data is encrypted on disk
func TestInFlightIngotStore_Recovery() // Crash recovery works
func TestCrashSimulation_AllDataRecovered() // Restart after write
```

### Integration Tests (Not yet written)
```python
# test_encryption_e2e.py
- Boot Mint with new master key
- Create 100 Phase3RoboTorqUnits
- Stop Mint (simulated crash)
- Verify SQLite file is encrypted (random binary data)
- Restart Mint with same key
- Verify all 100 units readable
- Attempt restart with wrong key (should fail)
```

### Performance Tests (Not yet written)
```
Baseline:
├─ JSON write: 1ms per unit
├─ JSON read: 0.5ms per unit

Expected with SQLite + encryption:
├─ Encrypted write: 2-3ms per unit
├─ Encrypted read: 1-2ms per unit
└─ Acceptable if <= 10x slower
```

---

## Deployment Checklist

- [ ] Master key generated and stored in GitHub Secrets
- [ ] Kyber private key generated and stored separately
- [ ] SQLite schema tested with migration tool
- [ ] Encryption layer passes all crypto tests
- [ ] Crash recovery tested (kill -9 and restart)
- [ ] Key rotation tested (new key, re-encrypt all data)
- [ ] Performance benchmarked (< 10x slower than JSON acceptable)
- [ ] Documentation updated (operations runbook)
- [ ] Disaster recovery plan written (key loss, corruption)
- [ ] Security audit passed (before production)
- [ ] Monitoring added (encryption errors, key rotation status)

---

## References

- **NIST Post-Quantum Cryptography**: https://csrc.nist.gov/projects/post-quantum-cryptography
- **liboqs**: https://github.com/open-quantum-safe/liboqs
- **AES-256-GCM**: NIST SP 800-38D (Recommendation for Block Cipher Modes)
- **SQLite Security**: https://www.sqlite.org/fileformat.html
- **Key Rotation Best Practices**: OWASP Cryptographic Failures (A02)

---

## Questions for Future Discussion

1. **Key rotation frequency**: Monthly, quarterly, or on-demand only?
2. **Backup strategy**: Keep encrypted backups? How many versions?
3. **Audit logging**: Log all decryption attempts? Performance impact?
4. **Multi-datacenter**: Different key per region? Global shared key?
5. **Compliance**: HIPAA, SOC 2, or other certification requirements?
6. **Disaster recovery**: RTO/RPO targets? Hot-standby Mint instances?

