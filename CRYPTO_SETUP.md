# Cryptography Setup Guide

This guide explains how to set up the cryptographic libraries required for Phase 5 signature verification.

---

## Overview

RoboTorq uses **post-quantum cryptography** for signature verification:

- **Falcon-1024**: Digger signs hash batches (Refinery verifies)
- **SPHINCS+**: Mint signs Phase3 RT units (archival signatures)

Both require **liboqs** (Open Quantum Safe C library).

---

## Installation

### Windows (MSYS2)

1. **Install MSYS2**:
   - Download from https://www.msys2.org/
   - Install to `C:\msys64`

2. **Install liboqs**:
   ```bash
   # Open MSYS2 MinGW 64-bit terminal
   pacman -S mingw-w64-x86_64-liboqs
   pacman -S mingw-w64-x86_64-pkg-config
   ```

3. **Add to PATH**:
   ```powershell
   # Add to system PATH
   $env:PATH += ";C:\msys64\mingw64\bin"
   $env:PKG_CONFIG_PATH = "C:\msys64\mingw64\lib\pkgconfig"
   ```

4. **Verify**:
   ```bash
   pkg-config --modversion liboqs
   # Should output: 0.11.0 or higher
   ```

### Linux (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential cmake git pkg-config

# Clone and build liboqs
git clone --branch main --single-branch --depth 1 https://github.com/open-quantum-safe/liboqs.git
cd liboqs
mkdir build && cd build
cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr/local ..
ninja
sudo ninja install
sudo ldconfig
```

### macOS (Homebrew)

```bash
brew install liboqs pkg-config
```

---

## Verification

Test that Go can find liboqs:

```bash
cd src/refinery
go test ./internal/crypto -v
```

Expected output:
```
=== RUN   TestVerifyHashBatch_InvalidHex
--- PASS: TestVerifyHashBatch_InvalidHex (0.00s)
=== RUN   TestVerifyHashBatch_ValidSignature
--- SKIP: TestVerifyHashBatch_ValidSignature (0.00s)
    falcon_test.go:12: TODO: Requires liboqs C library installation - see CRYPTO_SETUP.md
```

---

## Docker Build

The Dockerfile already includes liboqs installation:

```dockerfile
# Install liboqs for Falcon-1024 and SPHINCS+ verification
RUN apk add --no-cache \
    git \
    cmake \
    ninja \
    build-base \
    && git clone --branch main --depth 1 https://github.com/open-quantum-safe/liboqs.git /tmp/liboqs \
    && cd /tmp/liboqs \
    && mkdir build && cd build \
    && cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr .. \
    && ninja install \
    && rm -rf /tmp/liboqs
```

**Docker build works without local liboqs installation.**

---

## Troubleshooting

### Error: `pkg-config: executable file not found`

**Solution**: Install pkg-config:
```bash
# Windows (MSYS2)
pacman -S mingw-w64-x86_64-pkg-config

# Linux
sudo apt install pkg-config

# macOS
brew install pkg-config
```

### Error: `Package liboqs was not found in the pkg-config search path`

**Solution**: Set `PKG_CONFIG_PATH`:
```powershell
# Windows
$env:PKG_CONFIG_PATH = "C:\msys64\mingw64\lib\pkgconfig"

# Linux/macOS
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
```

### Error: `cannot find -loqs`

**Solution**: liboqs not installed. Follow installation steps above.

---

## CI/CD Configuration

GitHub Actions workflow already handles liboqs installation:

```yaml
- name: Install liboqs
  run: |
    git clone --branch main --depth 1 https://github.com/open-quantum-safe/liboqs.git
    cd liboqs
    mkdir build && cd build
    cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr/local ..
    ninja
    sudo ninja install
    sudo ldconfig

- name: Test with liboqs
  run: go test ./src/refinery/internal/crypto -v
```

---

## Development Workflow

### Option 1: Skip Verification Tests Locally (Faster)

```bash
go test ./... -short
```

Tests requiring liboqs are marked with `t.Skip()` when library not found.

### Option 2: Install liboqs Locally (Full Testing)

Follow installation steps above, then:

```bash
go test ./internal/crypto -v
```

### Option 3: Use Docker (Recommended)

```bash
docker-compose build refinery
docker-compose run refinery go test ./internal/crypto -v
```

---

## Phase 5 Status

- ✅ Falcon-1024 verification implemented (`falcon.go`)
- ✅ SPHINCS+ verification implemented (`sphincs.go`)
- ✅ Hex decoding validation working
- ⚠️ Full crypto tests require liboqs installation
- ✅ Docker builds include liboqs automatically

**For local development**: Docker is the easiest path. Tests will skip crypto verification if liboqs not found locally.

**For production**: Docker images include liboqs - no manual setup needed.
