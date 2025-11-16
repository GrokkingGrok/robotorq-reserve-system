# CI Migration - Digger Refactor

**Date**: November 16, 2025  
**Branch**: `feature/digger-refactor`  
**Status**: ✅ **COMPLETE**

---

## Overview

Migrated CI/CD pipeline from old Tauri desktop app to new Rust HTTP API with NATS integration.

**Old Architecture**:
- Tauri desktop app (src/digger-app)
- Node.js frontend + Rust backend
- GTK/WebKit dependencies
- .deb package artifact

**New Architecture**:
- Headless Rust HTTP API (src/digger)
- NATS integration for hash transmission
- Pure Rust (no Node.js)
- Binary artifact

---

## Changes Made

### 1. Updated `.github/workflows/ci.yml`

**Removed** (36 lines):
- Node.js setup
- Tauri CLI installation
- GTK/WebKit system dependencies
- `cargo tauri build` step
- .deb artifact upload

**Added** (30 lines):
- Digger-specific Cargo cache
- `cargo build --release` for src/digger
- `cargo test --all-features` for unit tests
- HTTP API health check test with NATS
- Binary artifact upload

### 2. Removed Old Code

**Deleted**:
- `src/digger-app/` directory (entire Tauri project)
  - digger/ (Tauri app)
  - test-digger-e2e.ps1
  - test-headless-quick.ps1

**Reason**: New Digger is canonical implementation.

---

## CI Test Matrix

### Go Services (Unchanged)
- ✅ Mint (port 8080)
- ✅ Refinery (port 8081)
- ✅ DistoDam (port 8082)
- ✅ Trust (port 8083)
- ✅ Wallet (port 3000)

### Digger (New)
- ✅ Build: `cargo build --release`
- ✅ Unit tests: `cargo test --all-features`
- ✅ Integration test: HTTP API + NATS health check
- ✅ Artifact: Upload `digger` binary

---

## CI Workflow Steps

```yaml
# 1. Setup Rust
- name: Install Rust stable
  uses: dtolnay/rust-toolchain@stable

# 2. Cache Cargo (Digger-specific)
- name: Cache Cargo registry and build
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      ~/.cargo/bin
      src/digger/target
    key: ${{ runner.os }}-digger-cargo-${{ hashFiles('src/digger/Cargo.lock') }}

# 3. Build Digger
- name: Build Digger
  working-directory: ./src/digger
  run: cargo build --release

# 4. Run Unit Tests
- name: Test Digger
  working-directory: ./src/digger
  run: cargo test --all-features

# 5. Test HTTP API
- name: Test Digger HTTP API
  run: |
    docker compose up -d nats
    sleep 5
    cd src/digger
    NATS_URL="nats://localhost:4222" BATCH_INTERVAL_SEC=10 cargo run --release &
    DIGGER_PID=$!
    sleep 15
    curl -f http://localhost:9000/health || exit 1
    kill $DIGGER_PID || true

# 6. Upload Binary
- name: Upload Digger Binary
  uses: actions/upload-artifact@v4
  with:
    name: digger-http-api-linux
    path: src/digger/target/release/digger
```

---

## Testing

### Local Verification

**Before pushing**:
```powershell
# Build
cd src/digger
cargo build --release

# Unit tests
cargo test --all-features

# Integration test (manual)
docker-compose up -d nats
$env:NATS_URL = "nats://localhost:4222"
cargo run --release

# Health check
curl http://localhost:9000/health
# Expected: {"status":"ok"}
```

### CI Verification

**After push**:
1. Push to `feature/digger-refactor`
2. Monitor GitHub Actions: https://github.com/USERNAME/robotorq-network/actions
3. Check all steps pass:
   - ✅ Build Digger
   - ✅ Test Digger (unit tests)
   - ✅ Test Digger HTTP API (integration)
   - ✅ Upload artifact

---

## Migration Checklist

- [x] Update CI workflow for new Digger
- [x] Remove Tauri dependencies (Node.js, GTK, WebKit)
- [x] Add Digger build step
- [x] Add Digger unit tests
- [x] Add Digger HTTP API test
- [x] Update artifact upload
- [x] Delete old digger-app directory
- [x] Commit CI changes
- [ ] Push to GitHub (pending)
- [ ] Verify CI passes on GitHub Actions
- [ ] Update README to reference new Digger
- [ ] Close old Digger issues/PRs

---

## Breaking Changes

**For Contributors**:
- Old Digger (Tauri) no longer exists
- New Digger is in `src/digger`
- No desktop UI - headless HTTP API only
- Test with `cargo test`, not Tauri commands

**For Deployment**:
- Binary artifact: `digger` (not .deb package)
- Run with: `./digger` (not desktop launcher)
- Configure via env vars (see README)

---

## Next Steps

### Immediate
- [ ] Push to GitHub: `git push origin feature/digger-refactor`
- [ ] Monitor CI: Verify all tests pass
- [ ] Update README: Document new Digger

### Future
- [ ] Phase 2: Refinery integration (consume NATS messages)
- [ ] Phase 4: Robot identity + Falcon-1024 signatures
- [ ] Phase 6: TOON binary encoding (60% bandwidth savings)

---

## Commit History

```
7c23899 ci: Update CI to test new Digger HTTP API, remove old Tauri app
4c0b34e feat(digger): Implement JTU creation and storage for NATS hash transmission
aaf2022 feat(digger): Add NATS hash transmission with background sender
163abba test(digger): Add comprehensive integration test for contract lifecycle
88e1948 docs(digger): Add TODO markers for Phase 4 robot identity tracking
722aae0 feat(digger): Add complete HTTP API with correct Torq economics
```

---

## Resources

- **CI Workflow**: `.github/workflows/ci.yml`
- **Digger Code**: `src/digger/`
- **NATS Test Guide**: `src/digger/NATS_TEST_GUIDE.md`
- **Architecture**: `src/digger/DIGGER_REWRITE_PLAN.md`
- **GitHub Actions**: https://github.com/USERNAME/robotorq-network/actions

---

## Success Criteria

**CI passes when**:
- ✅ All Go services build and pass health checks
- ✅ Digger builds with `cargo build --release`
- ✅ Unit tests pass: `cargo test --all-features`
- ✅ HTTP API responds to `/health` endpoint
- ✅ Binary artifact uploaded

**Manual verification**:
- ✅ 10,000 JTUs generated and stored (SQLite)
- ✅ 10,000 hashes transmitted to NATS ("ore.batch")
- ✅ User confirmed: "oh yeah I'm seeing all the hashes"

---

*"Watts > Wall Street"* 🤖⚡💰
