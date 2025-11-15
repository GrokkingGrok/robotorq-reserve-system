# GitHub Copilot Instructions - RoboTorq Network

**Project**: RoboTorq - Physics-Based Monetary System  
**Last Updated**: November 15, 2025  
**Purpose**: Guide AI agents through the proven development workflow

---

## 🎯 Project Overview

RoboTorq is **NOT** a cryptocurrency—it's a NATS-based distributed system where robotic labor creates measurable value through physics.

**Core Equation**: `1 RoboTorq = 1 kWh × 3600 tokens/sec × 1 hour`

**Architecture**: Message-passing system (NATS pub/sub), not blockchain
- **No chain**: Real-time message flows, not append-only ledger
- **No mining**: Value minted based on verified work
- **No gas fees**: Transaction fees fund operations (demurrage)

**Critical Reading**:
1. `README.md` (5,745 lines): Complete economic model, formulas, philosophy
2. `BRANCHING.md`: Git workflow (`v0` baseline, `feature/*` branches)
3. Service-specific architecture docs:
   - `src/mint/MINT_ARCHITECTURE.md`
   - `src/refinery/REFINERY_ARCHITECTURE.md`
   - `src/trust/TRUST_ARCHITECTURE.md` (future)

---

## 🔄 The Power Workflow

This 14-step cycle achieved **95% completion** of the currency refactor in one focused sprint.

### Phase 1: Planning & Context

#### 1. **Branch Checkout**
```bash
# Always work on feature branches
git checkout -b feature/currency-refactor

# Or continue existing work
git checkout feature/your-feature
git pull origin feature/your-feature
```

**Why**: Isolates work, enables parallel development, protects `main` from breaking changes.

#### 2. **Architecture Review**
Read relevant architecture docs BEFORE coding:
```bash
# For Mint work
cat src/mint/MINT_ARCHITECTURE.md

# For Refinery work
cat src/refinery/REFINERY_ARCHITECTURE.md

# For cross-service changes
grep -r "data flow" src/*/ARCHITECTURE.md
```

**Focus Areas**:
- Component responsibilities (what does X do?)
- Data flow diagrams (how do services communicate?)
- Key data structures (JouleTorqUnit, TokenTorqIngot, RoboTorqBatch)
- Configuration patterns (env vars, defaults)

**Example from Currency Refactor**:
> "Mint aggregates 1000 ingots OR flushes after 60s, builds merkle tree from ingot hashes, publishes batch to DistoDam."

#### 3. **Code Pattern Analysis**
Study existing implementations to match patterns:
```bash
# Example: Understanding Mint's components
find src/mint/internal/mint -name "*.go" | xargs head -50

# Look for:
# - Constructor patterns (New*())
# - Interface definitions
# - Error handling
# - Logging style
# - Metrics instrumentation
```

**Key Patterns in RoboTorq**:
- **Dependency Injection**: Components passed via constructors, not globals
  ```go
  func NewMintEngine(logger *slog.Logger, metrics *Metrics) *MintEngine
  ```

- **Graceful Shutdown**: All services respect `context.Context`
  ```go
  func (s *Service) Start(ctx context.Context) {
      for {
          select {
          case <-ctx.Done():
              s.drainBuffer()  // Flush pending work
              return
          // ...
          }
      }
  }
  ```

- **Structured Logging**: JSON logs with contextual fields
  ```go
  slog.Info("ingot assembled",
      "ingot_id", ingot.ID,
      "units", len(ingot.Units),
      "joules", ingot.JouleTorqTotal)
  ```

- **Prometheus Metrics**: Every operation instrumented
  ```go
  type Metrics struct {
      IngotsReceivedTotal prometheus.Counter
      ProcessingLatency   prometheus.Histogram
  }
  
  m.IngotsReceivedTotal.Inc()
  ```

---

### Phase 2: Implementation

#### 4. **Create TODOs in Code**
Mark implementation points BEFORE building:
```go
// TODO(currency-refactor): Replace dual queues with single unit queue
// - Remove: jouleQueue, roboQueue
// - Add: unitQueue []JouleTorqUnit
// - Update: processOre() to extract units
// - Fix: assembleIngot() to consume units
// - Test: Multi-contract unit aggregation

type Refinery struct {
    // OLD
    jouleQueue []float64  // TODO: REMOVE
    roboQueue  []float64  // TODO: REMOVE
    
    // NEW
    unitQueue  []*JouleTorqUnit  // TODO: IMPLEMENT
}
```

**Why**: Creates roadmap, enables incremental commits, documents intent.

**Naming Convention**:
- `TODO(feature-name):` - Planned work on feature branch
- `FIXME(issue-123):` - Bug fix for GitHub issue
- `WIP(component):` - Work in progress, incomplete
- `HACK:` - Temporary workaround, needs cleanup

#### 5. **Execute TODOs (Implementation)**
Tackle one TODO at a time, test as you go:

**Example: Refinery Single Queue Implementation**

```go
// Step 1: Add new data structure
type QueueManager struct {
    units    []*models.JouleTorqUnit
    mu       sync.Mutex
    notEmpty *sync.Cond
}

// Step 2: Implement AddUnit
func (qm *QueueManager) AddUnit(unit *models.JouleTorqUnit) error {
    qm.mu.Lock()
    defer qm.mu.Unlock()
    
    qm.units = append(qm.units, unit)
    qm.notEmpty.Signal()
    return nil
}

// Step 3: Implement GetUnit (blocking)
func (qm *QueueManager) GetUnit() (*models.JouleTorqUnit, error) {
    qm.mu.Lock()
    defer qm.mu.Unlock()
    
    for len(qm.units) == 0 {
        qm.notEmpty.Wait()  // Block until unit available
    }
    
    unit := qm.units[0]
    qm.units = qm.units[1:]
    return unit, nil
}
```

**Incremental Testing**:
```bash
# Test each function as implemented
go test ./internal/refinery/queue_manager_test.go -run TestAddUnit -v
go test ./internal/refinery/queue_manager_test.go -run TestGetUnit -v
go test ./internal/refinery/queue_manager_test.go -run TestConcurrency -v
```

#### 6. **Create Tests (TDD Approach)**
Write tests ALONGSIDE implementation, not after:

**Unit Test Example** (`queue_manager_test.go`):
```go
func TestQueueManager_AddAndGet(t *testing.T) {
    qm := NewQueueManager(100)
    
    unit := &models.JouleTorqUnit{
        TokenID: "test-token-001",
        JoulesConsumed: 4.17,
    }
    
    // Test add
    err := qm.AddUnit(unit)
    assert.NoError(t, err)
    assert.Equal(t, 1, qm.Len())
    
    // Test get
    retrieved, err := qm.GetUnit()
    assert.NoError(t, err)
    assert.Equal(t, "test-token-001", retrieved.TokenID)
    assert.Equal(t, 0, qm.Len())
}

func TestQueueManager_BlockingGet(t *testing.T) {
    qm := NewQueueManager(100)
    
    retrieved := false
    go func() {
        unit, _ := qm.GetUnit()  // Blocks
        assert.Equal(t, "async-token", unit.TokenID)
        retrieved = true
    }()
    
    time.Sleep(100 * time.Millisecond)
    assert.False(t, retrieved, "Should still be blocked")
    
    qm.AddUnit(&models.JouleTorqUnit{TokenID: "async-token"})
    
    time.Sleep(100 * time.Millisecond)
    assert.True(t, retrieved, "Should have unblocked")
}
```

**Coverage Target**: 95%+ for all new code
```bash
go test ./... -cover

# Expected output:
# ok      refinery/internal/refinery  0.450s  coverage: 96.2% of statements
```

---

### Phase 3: Validation

#### 7. **Step-by-Step Commits**
Commit after EACH working increment (not at end of day):

**Commit Pattern**:
```bash
# After implementing QueueManager AddUnit
git add internal/refinery/queue_manager.go
git add internal/refinery/queue_manager_test.go
git commit -m "feat(refinery): Add QueueManager.AddUnit with thread safety

- Implements unit storage in slice
- Uses mutex for goroutine safety
- Signals condition variable on add
- Test: concurrent adds from 10 goroutines"

# After implementing GetUnit
git add internal/refinery/queue_manager.go
git add internal/refinery/queue_manager_test.go
git commit -m "feat(refinery): Add QueueManager.GetUnit with blocking

- Blocks when queue empty (no busy-waiting)
- Uses sync.Cond for efficient wakeup
- Thread-safe pop operation
- Test: blocking behavior verified"

# After integration test
git add internal/refinery/integration_test.go
git commit -m "test(refinery): Add QueueManager integration test

- Tests full ore → unit → queue flow
- Validates concurrent AddUnit/GetUnit
- Confirms FIFO ordering
- Coverage: 100% of QueueManager"
```

**Commit Message Format** (Conventional Commits):
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `test`: Adding/updating tests
- `refactor`: Code restructure (no behavior change)
- `docs`: Documentation only
- `chore`: Build, CI, dependencies

**Scopes**: `mint`, `refinery`, `trust`, `distodam`, `digger`, `wallet`, `printer`

**Real Example from Currency Refactor**:
```
feat(refinery): Implement unit-based ingot assembly

Replace dual joule/robo queues with single JouleTorqUnit queue:
- Add QueueManager with AddUnit/GetUnit
- Update IngotAssembler to accumulate units
- Build merkle branch hash in NewTokenTorqIngot
- Preserve complete proof chain (token → unit → ingot)

Breaking Change: Ore processing now creates 1 unit per token
instead of aggregating joules/robo separately.

Tests:
- queue_manager_test.go: 100% coverage
- ingot_assembler_test.go: 96% coverage
- integration_test.go: Full pipeline verified

Refs: feature/currency-refactor, README.md Appendix O
```

#### 8. **Conduct E2E Tests**
Validate COMPLETE pipeline after changes:

**RoboTorq E2E Test** (`test-digger-e2e.ps1`):
```powershell
# Start all services
docker-compose up -d

# Wait for health
Start-Sleep -Seconds 10

# Build headless Digger
cd src/digger-app/digger
cargo build --release --bin headless

# Run headless in background
Start-Process -NoNewWindow -FilePath "./target/release/headless.exe"

# Send stake + execute contract
Invoke-RestMethod "http://localhost:9000/stake" -Method Post -Body '{"amount":0.05}' -ContentType "application/json"
Invoke-RestMethod "http://localhost:9000/execute" -Method Post -Body '{"contract_id":"e2e-test"}' -ContentType "application/json"

# Wait for processing (13 ores @ 5s each = 65s)
Write-Host "⏳ Waiting 65 seconds for ore processing..."
Start-Sleep -Seconds 65

# Verify Refinery logs
$logs = docker logs robotorq-network-refinery-1 --since 70s

if ($logs -match "ingot assembled") {
    Write-Host "✅ SUCCESS: Ingot assembled!" -ForegroundColor Green
    
    # Extract details
    $ingotLine = $logs | Select-String "ingot assembled" | Select-Object -Last 1
    Write-Host $ingotLine
} else {
    Write-Host "❌ FAILURE: No ingot found" -ForegroundColor Red
    exit 1
}

# Cleanup
Stop-Process -Name "headless" -Force
docker-compose down
```

**Expected Output**:
```
⏳ Waiting 65 seconds for ore processing...
✅ SUCCESS: Ingot assembled!

{"msg":"ingot assembled",
 "ingot_id":"ingot-20251115-210012.547794",
 "joules":14999.999999999249,
 "robo_stake":0.050000000000002334,
 "units":3600,
 "contracts":["e2e-test-contract-001"],
 "branch_hash":"0521434e13fa9bddc71d777d689635f73b1d91df68b74abcbd97af58a051ff10"}
```

**What E2E Test Validates**:
- ✅ Digger executes contract, sends ore
- ✅ Refinery receives ore, extracts units
- ✅ QueueManager stores units
- ✅ IngotAssembler accumulates 3600 units
- ✅ NewTokenTorqIngot builds merkle hash
- ✅ BatchSender publishes to Mint

#### 9. **Update Documentation**
Sync docs with implementation BEFORE merging:

**Architecture Doc Updates**:
```markdown
## Component Architecture

### QueueManager - Single Unit Queue ✅ UPDATED

**Responsibilities**:
- Store JouleTorqUnits in thread-safe FIFO queue
- Block on GetUnit() when empty (no busy-waiting)
- Track queue depth metrics

**Architecture Change** (Currency Refactor):
- ❌ OLD: Dual queues (JouleQueue + RoboQueue) - lost token granularity
- ✅ NEW: Single queue ([]JouleTorqUnit) - preserves complete proof chain

**Implementation**:
[...code examples...]

**Tests**: `queue_manager_test.go` (100% coverage)
```

**README Updates**:
```markdown
## Appendix O: Data Structures

### JouleTorqUnit ✅ UPDATED

The **atomic unit** of the RoboTorq system. Every token becomes one unit.

```go
type JouleTorqUnit struct {
    TokenID        string    `json:"token_id"`         // Unique: {contract}-m{milestone}-t{index}
    ContractID     string    `json:"contract_id"`      
    DiggerID       string    `json:"digger_id"`        
    JoulesConsumed float64   `json:"joules_consumed"`  // Energy for THIS token
    RoboStakePaid  float64   `json:"robo_stake_paid"`  // Cost of THIS token
    MilestoneIndex int       `json:"milestone_index"`  
    Timestamp      time.Time `json:"timestamp"`        
    Hash           string    `json:"hash"`             // SHA256 of all fields
    Signature      []byte    `json:"signature"`        // Dilithium5 signature
}
```

**Flow**: JouleTorqOre (300 tokens) → 300 JouleTorqUnits → accumulate to 3600 → TokenTorqIngot
```

**Commit Docs with Code**:
```bash
git add src/refinery/REFINERY_ARCHITECTURE.md
git add README.md
git commit -m "docs(refinery): Update architecture for unit-based queue

- Document QueueManager single-queue design
- Explain unit extraction from ore
- Add merkle hash construction details
- Update Appendix O with JouleTorqUnit flow"
```

---

### Phase 4: Integration

#### 10. **Update Build and CI**
Ensure CI can build and test your changes:

**Update Dockerfile** (if dependencies changed):
```dockerfile
FROM golang:1.24-alpine AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download  # ← Downloads new dependencies

COPY . .
RUN CGO_ENABLED=0 go build -o refinery ./cmd/refinery

# Tests run in CI
RUN go test ./... -cover

FROM alpine:3.20
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/refinery .
HEALTHCHECK --interval=30s CMD wget -qO- http://localhost:8081/health || exit 1
CMD ["./refinery"]
```

**Update `docker-compose.yaml`** (if new env vars):
```yaml
services:
  refinery:
    build: ./src/refinery
    environment:
      - NATS_URL=nats://nats:4222
      - REFINERY_JOULE_QUEUE_SIZE=1000        # ← NEW
      - REFINERY_INGOT_BATCH_INTERVAL=60      # ← NEW
      - LOG_LEVEL=info
    depends_on:
      - nats
```

**Verify Local Build**:
```bash
# Build image
cd src/refinery
docker build -t refinery:test .

# Run tests in container
docker run --rm refinery:test go test ./... -v

# Start service
docker-compose up -d refinery

# Check health
curl http://localhost:8081/health
```

#### 11. **Push to GitHub for Testing**
Trigger CI on feature branch:

```bash
# Ensure all tests pass locally first
go test ./... -v
pwsh test-digger-e2e.ps1

# Push feature branch
git push origin feature/currency-refactor
```

**GitHub Actions CI** (`.github/workflows/ci.yml`):
```yaml
name: CI

on:
  push:
    branches: [ main, 'feature/**' ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Set up Go
      uses: actions/setup-go@v4
      with:
        go-version: '1.24'
    
    - name: Run tests
      working-directory: src/refinery
      run: |
        go test ./... -v -cover
        go test -race ./...  # Race condition detection
    
    - name: Build Docker image
      run: docker build -t refinery:ci ./src/refinery
    
    - name: Start services
      run: docker-compose up -d
    
    - name: E2E test
      run: |
        sleep 30  # Wait for services
        ./test-digger-e2e.ps1
    
    - name: Cleanup
      run: docker-compose down
```

**Monitor CI**:
- Check GitHub Actions tab for workflow status
- Review test output for failures
- Fix any CI-specific issues (timeouts, dependencies)

#### 12. **Merge to Main**
Once CI passes on feature branch:

```bash
# Create pull request (GitHub CLI)
gh pr create \
  --title "Currency Refactor: Unit-Based Architecture" \
  --body "Implements single-queue unit storage, merkle branch hashes, and complete proof chain preservation.

## Changes
- Refinery: QueueManager with single unit queue
- Models: JouleTorqUnit, TokenTorqIngot with merkle hashes
- Mint: Ingot validation for 3600 units
- Tests: 95%+ coverage across all components

## Testing
- Unit tests: 96% coverage
- Integration tests: Full pipeline verified
- E2E test: Digger → Refinery → Mint flow working
- Performance: 300 units/sec throughput

## Breaking Changes
- Ore processing now creates 1 unit per token
- Ingots require exactly 3600 units (was variable joules)

## Migrations
None - fresh deployment

Closes #42" \
  --base main \
  --head feature/currency-refactor

# Wait for PR checks to pass
gh pr checks

# Merge (requires approval in production)
gh pr merge --squash --delete-branch
```

**Or via GitHub Web UI**:
1. Navigate to Pull Requests
2. Click "New Pull Request"
3. Select `feature/currency-refactor` → `main`
4. Add description (use template above)
5. Request reviews from team
6. Wait for approvals + CI green
7. Squash and merge

#### 13. **Push Main, Wait for CI**
Final validation on `main` branch:

```bash
# Pull merged changes
git checkout main
git pull origin main

# Verify version tag
git tag v1.0.0-currency-refactor
git push origin v1.0.0-currency-refactor

# Monitor CI on main
gh run watch
```

**Post-Merge Checklist**:
- ✅ CI passes on `main`
- ✅ Docker images built and tagged
- ✅ Documentation updated (README, architecture docs)
- ✅ Release notes published (if applicable)
- ✅ Stakeholders notified

---

## 📋 Quick Reference Checklists

### Starting New Feature
- [ ] Checkout feature branch: `git checkout -b feature/your-feature`
- [ ] Read relevant `ARCHITECTURE.md` files
- [ ] Review existing code patterns in target service
- [ ] Create implementation plan (TODO comments)
- [ ] Set up local testing environment: `docker-compose up -d`

### Before Each Commit
- [ ] Run unit tests: `go test ./... -v`
- [ ] Check coverage: `go test ./... -cover` (target: 95%+)
- [ ] Run integration tests (if applicable)
- [ ] Verify code compiles: `go build ./...`
- [ ] Check for race conditions: `go test -race ./...`
- [ ] Format code: `go fmt ./...`
- [ ] Lint: `golangci-lint run`

### Before Pushing
- [ ] All commits follow conventional format
- [ ] Run E2E test: `pwsh test-digger-e2e.ps1`
- [ ] Update architecture docs (if design changed)
- [ ] Update README (if public API changed)
- [ ] Update `docker-compose.yaml` (if config changed)
- [ ] Verify Docker build: `docker build .`
- [ ] Check for sensitive data (no secrets in code!)

### Before Merging
- [ ] CI passing on feature branch
- [ ] Code review approved (if team workflow)
- [ ] Documentation updated and reviewed
- [ ] Breaking changes documented
- [ ] Migration plan (if needed)
- [ ] Performance benchmarks (if critical path)

---

## 🔍 Code Review Guidelines

### What to Look For

**Architecture**:
- Does code follow service boundaries?
- Are dependencies injected (not global)?
- Is concurrency handled safely (mutexes, channels)?

**Testing**:
- Unit tests for all new functions?
- Coverage ≥95%?
- Edge cases tested (empty input, overflow, nil)?
- Integration tests for cross-component flows?

**Observability**:
- Structured logging with context?
- Prometheus metrics instrumented?
- Errors logged with stack traces?

**Documentation**:
- Architecture docs updated?
- Code comments explain WHY, not what?
- Public APIs documented?

**Performance**:
- No busy-waiting (use channels/conditions)?
- Bounded queues (no unbounded growth)?
- Graceful degradation under load?

### Review Comments Format

**Good**:
```
LGTM! QueueManager uses sync.Cond perfectly for blocking.

Minor: Consider adding a context.Context to GetUnit() for 
cancellation during shutdown. See Mint's IngotBuffer.Pop() 
for reference.
```

**Bad**:
```
This doesn't work.
```

---

## 🛠️ Common Patterns & Anti-Patterns

### ✅ DO: Dependency Injection
```go
// Good: Dependencies passed to constructor
type MintEngine struct {
    logger  *slog.Logger
    metrics *Metrics
    hasher  HashFunction
}

func NewMintEngine(logger *slog.Logger, metrics *Metrics, hasher HashFunction) *MintEngine {
    return &MintEngine{
        logger:  logger,
        metrics: metrics,
        hasher:  hasher,
    }
}
```

### ❌ DON'T: Global State
```go
// Bad: Global variables
var globalLogger *slog.Logger
var globalMetrics *Metrics

type MintEngine struct{}

func (me *MintEngine) Process() {
    globalLogger.Info("processing")  // Hard to test!
}
```

### ✅ DO: Context-Aware Blocking
```go
// Good: Respects cancellation
func (qm *QueueManager) GetUnit(ctx context.Context) (*JouleTorqUnit, error) {
    qm.mu.Lock()
    defer qm.mu.Unlock()
    
    for len(qm.units) == 0 {
        select {
        case <-ctx.Done():
            return nil, ctx.Err()
        default:
            qm.notEmpty.Wait()
        }
    }
    
    return qm.units[0], nil
}
```

### ❌ DON'T: Busy-Waiting
```go
// Bad: Burns CPU
func (qm *QueueManager) GetUnit() (*JouleTorqUnit, error) {
    for len(qm.units) == 0 {
        time.Sleep(10 * time.Millisecond)  // Wasteful!
    }
    return qm.units[0], nil
}
```

### ✅ DO: Structured Logging
```go
// Good: Contextual fields
slog.Info("ingot assembled",
    "ingot_id", ingot.ID,
    "units", len(ingot.Units),
    "joules", ingot.JouleTorqTotal,
    "contracts", ingot.ContractIDs)
```

### ❌ DON'T: Printf Debugging
```go
// Bad: Unstructured, no filtering
fmt.Printf("Ingot: %v\n", ingot)
```

### ✅ DO: Granular Metrics
```go
// Good: Specific counters
type Metrics struct {
    IngotsReceivedTotal    prometheus.Counter
    IngotsValidTotal       prometheus.Counter
    IngotsInvalidTotal     prometheus.Counter
    ProcessingLatency      prometheus.Histogram
}

m.IngotsReceivedTotal.Inc()
if err := validate(ingot); err != nil {
    m.IngotsInvalidTotal.Inc()
} else {
    m.IngotsValidTotal.Inc()
}
```

### ❌ DON'T: Sparse Metrics
```go
// Bad: Can't diagnose issues
type Metrics struct {
    RequestsTotal prometheus.Counter  // Too generic
}
```

---

## 🧪 Testing Philosophy

### Test Pyramid

```
         ┌─────────┐
         │   E2E   │  10% - Full pipeline, slow, brittle
         │  Tests  │
         └─────────┘
       ┌─────────────┐
       │ Integration │  20% - Multi-component, moderate speed
       │    Tests    │
       └─────────────┘
     ┌─────────────────┐
     │   Unit Tests    │  70% - Single function, fast, reliable
     └─────────────────┘
```

### Unit Test Example
```go
// Test ONE function in isolation
func TestQueueManager_AddUnit(t *testing.T) {
    qm := NewQueueManager(10)
    
    unit := &models.JouleTorqUnit{TokenID: "test-1"}
    
    err := qm.AddUnit(unit)
    
    assert.NoError(t, err)
    assert.Equal(t, 1, qm.Len())
}
```

### Integration Test Example
```go
// Test multiple components together
func TestRefinery_OreToIngot(t *testing.T) {
    // Setup
    qm := NewQueueManager(1000)
    assembler := NewIngotAssembler(qm)
    receiver := NewOreReceiver(qm)
    
    // Send ore
    ore := &models.JouleTorqOre{TokensGenerated: 300, Joules: 1250}
    receiver.ReceiveOre(ore)
    
    // Verify units queued
    assert.Equal(t, 300, qm.Len())
    
    // Assemble ingot (needs 3600 units = 12 ores)
    for i := 0; i < 11; i++ {
        receiver.ReceiveOre(ore)
    }
    
    ingot := assembler.GetCompletedIngots()[0]
    assert.Equal(t, 3600, len(ingot.Units))
    assert.NotEmpty(t, ingot.BranchHash)
}
```

### E2E Test Example
See `test-digger-e2e.ps1` for full pipeline test.

---

## 📊 Performance Benchmarks

### Running Benchmarks
```bash
# All benchmarks
go test -bench=. -benchmem ./internal/refinery

# Specific benchmark
go test -bench=BenchmarkQueueManager -benchmem ./internal/refinery
```

### Example Benchmark
```go
func BenchmarkQueueManager_Throughput(b *testing.B) {
    qm := NewQueueManager(100000)
    
    unit := &models.JouleTorqUnit{TokenID: "bench"}
    
    b.ResetTimer()
    for i := 0; i < b.N; i++ {
        qm.AddUnit(unit)
    }
}

// Output:
// BenchmarkQueueManager_Throughput-8   5000000   250 ns/op   64 B/op   1 allocs/op
//                                       ^^^^^     ^^^^^^^     ^^^^^^    ^^^^^^^^^^^
//                                       ops       ns/op       bytes     allocations
```

### Performance Targets
- **Refinery**: 300 units/sec sustained
- **Mint**: 1 batch/60s (1000 ingots)
- **Memory**: <500MB per service
- **Latency**: p99 < 100ms for API calls

---

## 🚀 Real-World Example: Currency Refactor

This workflow achieved **95% completion** in one sprint. Here's how:

### Week 1: Planning
- Read `README.md` Appendices (data structures, formulas)
- Diagrammed current flow (dual queues)
- Identified issue: Lost token granularity in aggregation
- Designed solution: Single unit queue with merkle trees

### Week 2: Refinery Refactor
```bash
# Day 1: QueueManager
git checkout -b feature/currency-refactor
# - Implement AddUnit/GetUnit
# - Write unit tests (100% coverage)
# - Commit: "feat(refinery): Add QueueManager single unit queue"

# Day 2: Unit Extraction
# - Update OreReceiver.extractUnits()
# - Distribute joules/robo per token
# - Commit: "feat(refinery): Extract JouleTorqUnits from ore"

# Day 3: Ingot Assembly
# - Refactor IngotAssembler to consume units
# - Commit: "refactor(refinery): Use unit-based ingot assembly"

# Day 4: Merkle Hashes
# - Implement CalculateBranchHash()
# - Commit: "feat(models): Add merkle branch hash to ingots"

# Day 5: Integration Test
# - Write ore → unit → ingot pipeline test
# - Commit: "test(refinery): Add integration test for unit flow"
```

### Week 3: Mint Integration
```bash
# Day 1: Ingot Validation
# - Update Mint to expect 3600 units
# - Reject ingots without units array
# - Commit: "feat(mint): Validate ingot unit count (3600)"

# Day 2: Merkle Tree Aggregation
# - Build batch hash from ingot hashes
# - Commit: "feat(mint): Aggregate ingot merkle hashes"

# Day 3-4: E2E Test
# - Implement test-digger-e2e.ps1
# - Verify full pipeline
# - Commit: "test(e2e): Add Digger → Refinery → Mint test"

# Day 5: Documentation
# - Rebuild MINT_ARCHITECTURE.md
# - Rebuild REFINERY_ARCHITECTURE.md
# - Commit: "docs: Rebuild architecture for currency refactor"
```

### Week 4: Polish & Merge
```bash
# Day 1: Remove outdated TODOs
# Day 2: CI/CD updates
# Day 3: PR review, address feedback
# Day 4: Merge to main
# Day 5: Monitor production deploy
```

**Result**: **95% complete**, only crypto signatures remain (~12h work).

---

## 🔐 Security Reminders

- **No secrets in code**: Use environment variables
- **Validate all inputs**: Never trust external data
- **Sanitize logs**: Don't log sensitive data (private keys, passwords)
- **Use prepared statements**: (When SQL added in future)
- **Principle of least privilege**: Services only access what they need

---

## 📚 Additional Resources

- **White Paper**: `README.md` (complete economic model)
- **Branching Strategy**: `BRANCHING.md`
- **Mint Architecture**: `src/mint/MINT_ARCHITECTURE.md`
- **Refinery Architecture**: `src/refinery/REFINERY_ARCHITECTURE.md`
- **Status Tracker**: `CURRENCY_REFACTOR_STATUS.md`
- **Prometheus Dashboard**: `Grafana/torq-observability-dashboard.json`

---

## 🎓 Final Tips

1. **Read the white paper first**: The economics drive the architecture
2. **Test as you build**: Don't write 1000 lines then test
3. **Commit frequently**: Small commits are easier to review and revert
4. **Document decisions**: Future you will thank present you
5. **Ask for help**: Architecture docs exist for a reason
6. **Measure everything**: Metrics reveal truth
7. **Respect context**: Graceful shutdown prevents data loss

**Remember**: This is a **monetary system based on physics**. Every line of code represents real energy, real computation, and real value. Code accordingly.

*"Watts > Wall Street"* 🤖⚡💰
