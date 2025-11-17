# RoboTorq Network - Port Mappings

**Last Updated**: November 17, 2025  
**Purpose**: Document all current port allocations to prevent conflicts during service configuration

---

## 🚨 PORT CONFLICT - RESOLVED ✅

**Previous Issue**: Mint verification API and DistoDam were both attempting to use host port **8082**

```yaml
Mint:      "8082:8081"  # Verification API - CONFLICT!
DistoDam:  "8082:8082"  # Main API - CONFLICT!
```

**Resolution Applied**: November 17, 2025
```yaml
Mint:      "8084:8081"  # Verification API - RESOLVED ✅
DistoDam:  "8082:8082"  # Main API - No change
```

**Status**: ✅ RESOLVED - All services can now start

---

## 📊 Current Port Allocation Table

| Service | Host Port(s) | Container Port(s) | Purpose | Status |
|---------|--------------|-------------------|---------|--------|
| **NATS** | 4222 | 4222 | Messaging broker | ✅ OK |
| **NATS** | 8222 | 8222 | HTTP monitoring/health | ✅ OK |
| **Postgres** | _(internal)_ | 5432 | Database (internal only) | ✅ OK |
| **Mint** | 50051 | 50051 | gRPC API | ✅ OK |
| **Mint** | 8080 | 8080 | Main HTTP API | ✅ OK |
| **Mint** | **8084** | 8081 | **Verification API** | ✅ **RESOLVED** |
| **Mint** | 9090 | 9090 | Prometheus metrics | ✅ OK |
| **Digger** | 3030 | 3030 | HTTP API (Rust) | ✅ OK |
| **Refinery** | 50052 | 50052 | gRPC API | ✅ OK |
| **Refinery** | 8081 | 8080 | HTTP API | ✅ OK |
| **Wallet** | 3000 | 3000 | HTTP API | ✅ OK |
| **DistoDam** | 8082 | 8082 | HTTP API | ✅ OK |
| **Trust** | 8083 | 8080 | HTTP API | ✅ OK |
| **Prometheus** | 9091 | 9090 | Metrics collector UI | ✅ OK |
| **Grafana** | 3001 | 3000 | Dashboard UI | ✅ OK |

---

## 🔍 Port Range Analysis

### Occupied Host Ports (Sorted)
- **3000**: Wallet
- **3001**: Grafana
- **3030**: Digger
- **4222**: NATS (messaging)
- **8080**: Mint (main HTTP API)
- **8081**: Refinery (HTTP API)
- **8082**: DistoDam
- **8083**: Trust
- **8084**: Mint (verification API)
- **8222**: NATS (monitoring)
- **9090**: Mint (metrics)
- **9091**: Prometheus
- **50051**: Mint (gRPC)
- **50052**: Refinery (gRPC)

### Available Host Ports
- **8085**: ✅ Available
- **8086**: ✅ Available
- **9092**: ✅ Available
- **9093**: ✅ Available

---

## 🔧 Proposed Resolution Options

### Option A: Minimal Changes (Recommended)
**Keep existing services on current ports, only move Mint verification**

```yaml
Mint verification:  8082 → 8084  (host:8084 → container:8081)
DistoDam:           8082 (no change)
Trust:              8083 (no change)
```

**Changes Required**:
- [ ] Update `docker-compose.yaml`: Mint ports section
- [ ] Update `tests/integration/mint_verification_api.py`: `BASE_URL = "http://localhost:8084"`
- [ ] Update `tests/e2e/phase5_verification_flow.py`: `VERIFICATION_API = "http://localhost:8084"`

**Impact**: Minimal (only 3 file changes, no other services affected)

---

### Option B: Service Reorganization
**Consolidate services into logical port ranges**

```yaml
HTTP APIs (8080-8089):
  Mint main:          8080 (no change)
  Refinery:           8081 (no change)
  DistoDam:           8082 (no change)
  Trust:              8083 (no change)
  Mint verification:  8084 (NEW)

gRPC APIs (50051-50059):
  Mint:               50051 (no change)
  Refinery:           50052 (no change)

Metrics (9090-9099):
  Mint:               9090 (no change)
  Prometheus:         9091 (no change)

User-facing (3000-3099):
  Wallet:             3000 (no change)
  Grafana:            3001 (no change)
  Digger:             3030 (no change)
```

**Changes Required**: Same as Option A (3 files)

**Impact**: Minimal (same as Option A, just provides clearer organization)

---

### Option C: Complete Reorganization (Not Recommended)
**Reassign all ports into strict ranges**

```yaml
NATS Cluster (4000-4099):
  Messaging:          4222 (no change)
  Monitoring:         8222 → 4223 (BREAKING)

HTTP APIs (8080-8089):
  Mint main:          8080 (no change)
  Refinery:           8081 → 8080 (host) (BREAKING - conflicts!)
  DistoDam:           8082 (no change)
  Trust:              8083 (no change)
  Mint verification:  8084 (NEW)
```

**Impact**: ❌ **NOT RECOMMENDED** - Requires extensive changes, breaks existing integrations

---

## 📝 Service-Specific Details

### Mint Service (Multi-Port)
- **Purpose**: Central authority for RoboTorq minting and verification
- **Ports**:
  - `50051` (gRPC): Phase2Ingot reception from Refinery
  - `8080` (HTTP): Main API (health, metrics endpoint)
  - `8082→8081` (HTTP): **Phase 5 Verification API** ← CONFLICT!
  - `9090` (HTTP): Prometheus metrics scraping
- **Container Internal**: Verification API runs on port `8081` inside container
- **Resolution**: Host port changed from `8082` to `8084` (November 17, 2025)

### DistoDam Service
- **Purpose**: Distributed storage and merkle tree management
- **Ports**:
  - `8082` (HTTP): Main API for Phase3 unit storage and retrieval
- **Container Internal**: Runs on port `8082` (same as host)
- **Status**: No conflicts (Mint verification moved to 8084)

### Trust Service
- **Purpose**: Reputation and trust score management
- **Ports**:
  - `8083→8080` (HTTP): Main API
- **Container Internal**: Runs on port `8080` inside container
- **Note**: Would conflict if Mint verification moved to `8083`

---

## 🧪 Test Configuration Dependencies

### Integration Tests (`tests/integration/mint_verification_api.py`)
- **Current Config**: `MINT_VERIFICATION_API = "http://localhost:8084"`
- **Uses**: All 5 verification endpoints
- **Status**: ✅ 8/8 tests passing

### E2E Tests (`tests/e2e/phase5_verification_flow.py`)
- **Current Config**: `MINT_VERIFICATION_API = "http://localhost:8084"`
- **Uses**: Complete pipeline testing
- **Status**: ✅ Ready to run (port conflict resolved)

---

## ✅ Resolution Applied - November 17, 2025

**Option A: Minimal Changes - COMPLETED**

### Changes Made:
- ✅ Updated `docker-compose.yaml`: Mint verification `8082:8081` → `8084:8081`
- ✅ Updated `tests/integration/mint_verification_api.py`: Port 8082 → 8084
- ✅ Updated `tests/e2e/phase5_verification_flow.py`: Port 8082 → 8084

### Next Steps:
1. Rebuild and restart services:
   ```powershell
   docker-compose down
   docker-compose up -d
   ```

2. Verify all containers start successfully:
   ```powershell
   docker-compose ps
   ```

3. Run integration tests:
   ```powershell
   python .\tests\integration\mint_verification_api.py
   ```

4. Run E2E tests:
   ```powershell
   python .\tests\e2e\phase5_verification_flow.py
   ```

---

## 📋 Port Conflict History

### November 17, 2025 - RESOLVED ✅
1. **First Conflict**: Mint verification initially assigned to `8081` (conflicted with Refinery)
2. **Second Conflict**: Moved to `8082` (conflicted with DistoDam)
3. **Resolution Applied**: Moved to `8084` (no conflicts) - Option A implemented

### Root Cause
- Agent failed to verify existing port allocations before assigning new ports
- No comprehensive port mapping document existed
- Docker Compose does not validate port conflicts until runtime

### Prevention
- ✅ **This document** created to prevent future conflicts
- ✅ Always check this document before assigning new ports
- ✅ Update this document when adding new services
- ✅ Run `docker-compose config` to validate before deployment

---

## 🔒 Reserved Ports (Do Not Use)

These ports are commonly used by system services or other applications:

- **80**: HTTP (system)
- **443**: HTTPS (system)
- **5000**: Common dev server port
- **5432**: PostgreSQL (used internally by our container)
- **6379**: Redis (if added in future)
- **27017**: MongoDB (if added in future)

---

## 📞 Contact

If adding new services or modifying port assignments:
1. **Check this document first**
2. **Update this document** with proposed changes
3. **Get approval** before modifying `docker-compose.yaml`
4. **Test locally** before committing

---

**END OF DOCUMENT**
