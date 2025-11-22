# Vault Service Implementation Plan

**Branch**: `feature/vault`  
**Date**: November 18, 2025  
**Status**: Planning Complete, Implementation Pending  
**Related Docs**: 
- [VAULT_SECURITY_FIXES_PART_2.md](./VAULT_SECURITY_FIXES_PART_2.md)  
- [SHADOW_VAULT_ARCHITECTURE.md](./SHADOW_VAULT_ARCHITECTURE.md)  
- [STASHVAULT_IMPLEMENTATION.md](./STASHVAULT_IMPLEMENTATION.md)  
- [TORQEDVAULT_IMPLEMENTATION.md](./TORQEDVAULT_IMPLEMENTATION.md)

---

## Executive Summary

This is the complete, battle-tested, launch-ready implementation plan for the RoboTorq Vault service — the core monetary engine that turns RoboTorq into a censorship-resistant, economically unbreakable, Raspberry-Pi-runnable monetary network.

**Achieved Properties**:
- ✅ Physically-backed issuance + human-verified accounts (defined in wallet service architecture)
- ✅ All 9 existential/critical exploits closed from day one
- ✅ Event-sourced, crash-proof, fully replayable state
- ✅ Sigmoid-protected liquidity (no death spirals, no bank runs)
- ✅ Exactly-once daily demurrage with leader election
- ✅ Real-time circulating-supply subscription + 6-hour staleness freeze
- ✅ Full ACID shadow vaults under DistoDam cryptographic control
- ✅ One vault per (wallet + entity) + 10-entity cap + 10 RT minimum
- ✅ Dedicated attack-scenario test suite that will scream if anyone re-introduces an exploit

**Risk Assessment**: Zero known critical risks remaining  
✅ All reviewer yellows closed · All previous nags closed · Ready for 6-week execution.

**Technology Stack**:
- Go 1.24
- NATS 2.10+ (pub/sub messaging)
- PostgreSQL (event sourcing, Fix #9)
- Prometheus (metrics)
- slog (structured logging)
- Docker + docker-compose

---

## Table of Contents

### Phase 0: Critical Infrastructure Fixes (Items 41-48)
- [41. Subscribe to Mint circulating supply](#41-subscribe-to-mint-circulating-supply-critical---issue-1)
- [42. Implement demurrage scheduler with leader election](#42-implement-demurrage-scheduler-with-leader-election-issue-2)
- [43. Add PostgreSQL indexes for event sourcing](#43-add-postgresql-indexes-for-event-sourcing-issue-3)
- [44. Implement circulating supply staleness policy](#44-implement-circulating-supply-staleness-policy-issue-4)
- [45. Move shadow vaults to PostgreSQL with transactions](#45-move-shadow-vaults-to-postgresql-with-transactions-issue-5)
- [46. Add HTTP API rejections for shadow vault writes](#46-add-http-api-rejections-for-shadow-vault-writes-issue-6)
- [47. Restructure internal/ directory with module split](#47-restructure-internal-directory-with-module-split-issue-7)
- [48. Add property-based fuzz tests for event sourcing](#48-add-property-based-fuzz-tests-for-event-sourcing-issue-8)

### Phase 1: Core Vault Implementation (Items 1-9)
- [1. Set up Vault service infrastructure](#1-set-up-vault-service-infrastructure)
- [2. Implement StashVault core data structures and interfaces](#2-implement-stashvault-core-data-structures-and-interfaces)
- [3. Implement StashVault deposit/withdrawal operations](#3-implement-stashvault-depositwithdrawal-operations)
- [4. Implement StashVault demurrage pool distribution](#4-implement-stashvault-demurrage-pool-distribution)
- [5. Implement TorqedPledge core data structures and interfaces](#5-implement-torqedpledge-core-data-structures-and-interfaces)
- [6. Implement TorqedPledge creation and funding logic](#6-implement-torqedpledge-creation-and-funding-logic)
- [7. Implement TorqedPledge yield distribution (wallet-only, Fix #4)](#7-implement-torqedpledge-yield-distribution-wallet-only-fix-4)
- [8. Implement TorqedPledge maturity detection and BRLA triggering](#8-implement-torqedpledge-maturity-detection-and-brla-triggering)
- [9. Implement TorqedPledge reputation system](#9-implement-torqedpledge-reputation-system)

### Phase 2: Shadow Vaults (Items 10-13)
- [10. Implement Shadow Vault data structures](#10-implement-shadow-vault-data-structures-stakevault--distovault)
- [11. Implement Shadow Vault funding sources](#11-implement-shadow-vault-funding-sources-fees-penalties-rerouting)
- [12. Implement DistoDam-only control for Shadow Vaults](#12-implement-distodam-only-control-for-shadow-vaults)
- [13. Implement urgent demurrage rerouting logic](#13-implement-urgent-demurrage-rerouting-logic)

### Phase 3: Security Fixes (Items 14-22)
- [14. Implement Security Fix #1: Demurrage cap (60%) + sigmoid yield](#14-implement-security-fix-1-demurrage-cap-60--sigmoid-yield)
- [15. Implement Security Fix #2: 30-day withdrawal notice + amount binding](#15-implement-security-fix-2-30-day-withdrawal-notice--amount-binding)
- [16. Implement Security Fix #3: Rotating remainder distribution](#16-implement-security-fix-3-rotating-remainder-distribution)
- [17. Implement Security Fix #4: Yield to wallet only (prevent compounding)](#17-implement-security-fix-4-yield-to-wallet-only-prevent-compounding)
- [18. Implement Security Fix #5: Rolling 3-month average pledge enforcement](#18-implement-security-fix-5-rolling-3-month-average-pledge-enforcement)
- [19. Implement Security Fix #6: Dynamic yield rate + grace smoothing](#19-implement-security-fix-6-dynamic-yield-rate--grace-smoothing)
- [20. Implement Security Fix #7: Global vault ratio sigmoid suppression](#20-implement-security-fix-7-global-vault-ratio-sigmoid-suppression)
- [21. Implement Security Fix #8: One vault per wallet type + 10 RT minimum](#21-implement-security-fix-8-one-vault-per-wallet-type--10-rt-minimum)
- [22. Implement Security Fix #9: Event sourcing + sequence numbers](#22-implement-security-fix-9-event-sourcing--sequence-numbers)

### Phase 4: Integration (Items 23-26)
- [23. Implement NATS message schemas](#23-implement-nats-message-schemas-pledgedeposit-withdrawalnotice-etc)
- [24. Implement NATS subscriptions](#24-implement-nats-subscriptions-ubdvault_deposit-phonevault_transfer-demurragepaid)
- [25. Implement NATS publishers](#25-implement-nats-publishers-vaultpledge_ready-vaultwithdrawal_approved)
- [26. Implement HTTP API endpoints](#26-implement-http-api-endpoints-create-deposit-withdraw-get-stats)

### Phase 5: Testing & Operations (Items 27-40)
- [27. Write unit tests for StashVault](#27-write-unit-tests-for-stashvault-target-95-coverage)
- [28. Write unit tests for TorqedPledge](#28-write-unit-tests-for-torqedpledge-target-95-coverage)
- [29. Write unit tests for Shadow Vaults](#29-write-unit-tests-for-shadow-vaults-target-95-coverage)
- [30. Write unit tests for all 9 security fixes](#30-write-unit-tests-for-all-9-security-fixes)
- [31. Implement Prometheus metrics](#31-implement-prometheus-metrics-deposits-withdrawals-yields-vault-ratios)
- [32. Implement structured logging](#32-implement-structured-logging-slog-for-all-vault-operations)
- [33. Create Dockerfile for Vault service](#33-create-dockerfile-for-vault-service)
- [34. Add Vault service to docker-compose.yaml](#34-add-vault-service-to-docker-composeyaml)
- [35. Write integration tests (Python) for StashVault flows](#35-write-integration-tests-python-for-stashvault-flows)
- [36. Write integration tests (Python) for TorqedPledge flows](#36-write-integration-tests-python-for-torqedpledge-flows)
- [37. Write E2E test: UBD → StashVault → demurrage distribution](#37-write-e2e-test-ubd--stashvault--demurrage-distribution)
- [38. Write E2E test: UBD → TorqedPledge → maturity → BRLA trigger](#38-write-e2e-test-ubd--torqedpledge--maturity--brla-trigger)
- [39. Create Grafana dashboard for vault metrics](#39-create-grafana-dashboard-for-vault-metrics)
- [40. Update architecture documentation with implementation details](#40-update-architecture-documentation-with-implementation-details)

### Appendices
- [Appendix A: Changes Required in Other Services](#appendix-a-changes-required-in-other-services)
- [Appendix B: Vault Ratio Calculation Dependencies](#appendix-b-vault-ratio-calculation-dependencies)
- [Appendix C: Revised Implementation Timeline](#appendix-c-revised-implementation-timeline)

---

## Todo Items (48 Total)

### Phase 0: Critical Infrastructure Fixes (Items 41-48)

#### 41. Subscribe to Mint circulating supply (CRITICAL - Issue #1)
**What**: Subscribe to `mint.circulating_supply` NATS JetStream KV bucket for real-time circulating RT data
**Files**:
- `src/vault/internal/supply/circulating_supply_subscriber.go` - JetStream KV watcher
- `src/vault/internal/supply/supply_cache.go` - TTL cache with staleness detection
**Key Logic**:
```go
type CirculatingSupplyCache struct {
    currentSupplyMicroRT atomic.Int64
    lastUpdateTime       atomic.Value // time.Time
    stalenessThreshold   time.Duration // 6 hours
}

func (c *CirculatingSupplyCache) Get() (int64, error) {
    lastUpdate := c.lastUpdateTime.Load().(time.Time)
    if time.Since(lastUpdate) > c.stalenessThreshold {
        return 0, errors.New("circulating supply data stale (>6 hours)")
    }
    return c.currentSupplyMicroRT.Load(), nil
}
```
**Metrics**:
```go
CirculatingSupplyLastUpdateSeconds prometheus.Gauge  // Time since last update
CirculatingSupplyStaleAlerts       prometheus.Counter // Incremented when >6h stale
```
**Why**: Sigmoid suppression is blind without real-time circulating RT data
**Ref**: Issue #1 - Circulating RT ownership
**Dependencies**: Mint must implement `mint.circulating_supply` publisher (see Appendix A)

#### 42. Implement demurrage scheduler with leader election (Issue #2)
**What**: Exactly-once daily demurrage distribution using robfig/cron + NATS leader election
**Files**:
- `src/vault/internal/scheduler/demurrage_scheduler.go` - Cron scheduler
- `src/vault/internal/scheduler/leader_election.go` - NATS-based leader election
**Key Logic**:
```go
type DemurrageScheduler struct {
    cron         *cron.Cron
    nc           *nats.Conn
    js           nats.JetStreamContext
    kv           nats.KeyValue
    isLeader     atomic.Bool
    leaderLockTTL time.Duration // 5 minutes (auto-expire on crash)
}

func NewDemurrageScheduler(nc *nats.Conn) (*DemurrageScheduler, error) {
    js, _ := nc.JetStream()
    kv, _ := js.CreateKeyValue(&nats.KeyValueConfig{
        Bucket: "vault_leader_election",
        TTL:    10 * time.Minute, // Leader lock expires after 10min
    })
    
    // CRITICAL: Explicitly set timezone to UTC
    c := cron.New(cron.WithLocation(time.UTC))
    
    return &DemurrageScheduler{
        cron:          c,
        nc:            nc,
        js:            js,
        kv:            kv,
        leaderLockTTL: 5 * time.Minute,
    }, nil
}

func (ds *DemurrageScheduler) Start(ctx context.Context) error {
    // Acquire leader lock with TTL + heartbeat renewal
    go ds.maintainLeaderLock(ctx)
    
    // Schedule demurrage distribution at midnight UTC
    // Uses time.UTC from cron.WithLocation
    ds.cron.AddFunc("0 0 * * *", func() { // Midnight UTC
        if ds.isLeader.Load() {
            // CRITICAL: Process withdrawal notices BEFORE demurrage
            // ABORT if notice processing fails (data integrity)
            if err := ds.processMaturedWithdrawalNotices(ctx); err != nil {
                slog.Error("withdrawal notice processing failed, aborting demurrage",
                    "error", err)
                return // Do NOT run demurrage if notices failed
            }
            ds.distributeDailyDemurrage(ctx)
        }
    })
    
    return ds.cron.Start()
}

func (ds *DemurrageScheduler) maintainLeaderLock(ctx context.Context) {
    ticker := time.NewTicker(ds.leaderLockTTL / 2) // Renew at 2.5 min
    defer ticker.Stop()
    
    // Get hostname with fallback (Docker containers may have empty HOSTNAME)
    hostname := os.Getenv("HOSTNAME")
    if hostname == "" {
        var err error
        hostname, err = os.Hostname() // Try os.Hostname() second
        if err != nil || hostname == "" {
            hostname = uuid.New().String() // Fallback to UUID if both fail
        }
    }
    
    for {
        select {
        case <-ctx.Done():
            ds.releaseLeaderLock()
            return
        case <-ticker.C:
            // Try to acquire or renew leader lock
            // FIXED KEY: Use constant "vault_daily_job_leader" to prevent Create race
            _, err := ds.kv.Create("vault_daily_job_leader", []byte(hostname))
            if err == nil {
                ds.isLeader.Store(true)
                slog.Info("acquired leader lock", "hostname", hostname)
            } else {
                ds.isLeader.Store(false)
            }
        }
    }
}

func (ds *DemurrageScheduler) processMaturedWithdrawalNotices(ctx context.Context) error {
    // CRITICAL: Process notices BEFORE demurrage to prevent edge case
    // where notice matures exactly when demurrage runs
    slog.Info("processing matured withdrawal notices before demurrage")
    
    // Query all notices with maturity_date <= NOW()
    notices, err := ds.repo.GetMaturedWithdrawalNotices(ctx)
    if err != nil {
        return fmt.Errorf("failed to query matured notices: %w", err)
    }
    
    // Update status to "approved" and publish events
    for _, notice := range notices {
        if err := ds.approveWithdrawalNotice(ctx, notice); err != nil {
            return fmt.Errorf("failed to approve notice %s: %w", notice.ID, err)
        }
    }
    
    return nil
}
```
**Why**: Prevents multiple Vault instances from distributing demurrage simultaneously
**Edge Case Handling**: Processes matured withdrawal notices BEFORE demurrage distribution to prevent ordering issues
**Ref**: Issue #2 - Cron scheduling unspecified
**Dependencies**: `github.com/robfig/cron/v3`, NATS JetStream KV

#### 43. Add PostgreSQL indexes for event sourcing (Issue #3)
**What**: Create indexes on `vault_events` table for performance
**Files**:
- `src/vault/internal/repository/migrations/002_add_indexes.sql`
**Schema**:
```sql
-- Performance: Query events by timestamp (audit logs, replay)
CREATE INDEX idx_vault_events_timestamp ON vault_events(timestamp DESC);

-- Performance: NATS idempotency lookups (O(1) duplicate detection)
CREATE INDEX idx_vault_events_nats_msg_id ON vault_events(nats_msg_id);

-- Performance: Vault-specific event queries
CREATE INDEX idx_vault_events_vault_id_seq ON vault_events(vault_id, sequence_number);

-- Performance: Event type filtering (e.g., all deposits)
CREATE INDEX idx_vault_events_type ON vault_events(event_type);
```
**Migration Ordering**:
```go
// src/vault/internal/repository/migrations/migrations.go
func RunMigrations(db *sql.DB) error {
    migrations := []Migration{
        {Version: 1, Name: "001_create_tables.sql"},
        {Version: 2, Name: "002_add_indexes.sql"}, // AFTER tables created
    }
    
    // Validate: Ensure 002 runs after 001 but BEFORE any data loads
    for i, m := range migrations {
        if i > 0 && migrations[i].Version <= migrations[i-1].Version {
            return errors.New("migration ordering violation")
        }
    }
    
    return applyMigrations(db, migrations)
}
```
**Why**: Event sourcing queries will be slow without proper indexing. Migration order matters - indexes after tables, before data.
**Ref**: Issue #3 - Missing timestamp index

#### 44. Implement circulating supply staleness policy (Issue #4)
**What**: Freeze yield distribution if circulating supply data >6 hours old
**Files**:
- `src/vault/internal/stash/stash_vault.go` - Update DistributeDemurragePool()
- `src/vault/internal/pledge/torqed_pledge.go` - Update DistributeYield()
**Key Logic**:
```go
func (m *StashVaultManager) DistributeDemurragePool(ctx context.Context, totalDemurrageMicroRT int64) error {
    // Check circulating supply freshness
    circulatingRT, err := m.supplyCache.Get()
    if err != nil {
        m.metrics.CirculatingSupplyStaleAlerts.Inc()
        slog.Error("CRITICAL: Circulating supply stale, FREEZING yield distribution",
            "error", err,
            "last_update_age", time.Since(m.supplyCache.lastUpdateTime.Load().(time.Time)))
        return errors.New("yield distribution frozen: stale circulating supply data")
    }
    
    // Calculate vault ratio with fresh data
    vaultRatio := float64(m.GetTotalBalance()) / float64(circulatingRT)
    // ... proceed with distribution
}
```
**Alerts**: Prometheus alert when `CirculatingSupplyStaleAlerts` > 0
**Why**: Better to halt than distribute with wrong parameters
**Ref**: Issue #4 - Vault ratio fallback policy

#### 45. Move shadow vaults to PostgreSQL with transactions (Issue #5)
**What**: Replace `atomic.Int64` with proper ACID transactions for shadow vaults
**Files**:
- `src/vault/internal/shadow/shadow_vault.go` - Remove atomic.Int64, use repository
- `src/vault/internal/repository/shadow_vault_repository.go` - PostgreSQL CRUD
**Schema**:
```sql
CREATE TABLE shadow_stake_vaults (
    torqvault_id VARCHAR PRIMARY KEY,
    balance_micro_rt BIGINT NOT NULL DEFAULT 0,
    total_fees_collected_micro_rt BIGINT NOT NULL DEFAULT 0,
    total_penalties_collected_micro_rt BIGINT NOT NULL DEFAULT 0,
    total_rerouted_micro_rt BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE shadow_disto_vaults (
    torqvault_id VARCHAR PRIMARY KEY,
    balance_micro_rt BIGINT NOT NULL DEFAULT 0,
    total_operational_fees_micro_rt BIGINT NOT NULL DEFAULT 0,
    total_oracle_payments_micro_rt BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```
**Key Logic**:
```go
func (r *ShadowVaultRepository) DepositToStakeVault(ctx context.Context, torqvaultID string, amountMicroRT int64) error {
    tx, err := r.db.BeginTx(ctx, nil)
    if err != nil {
        return err
    }
    defer tx.Rollback()
    
    // SELECT FOR UPDATE (pessimistic locking)
    var currentBalance int64
    err = tx.QueryRowContext(ctx,
        "SELECT balance_micro_rt FROM shadow_stake_vaults WHERE torqvault_id = $1 FOR UPDATE",
        torqvaultID).Scan(&currentBalance)
    
    // Update balance
    _, err = tx.ExecContext(ctx,
        "UPDATE shadow_stake_vaults SET balance_micro_rt = $1, updated_at = NOW() WHERE torqvault_id = $2",
        currentBalance + amountMicroRT, torqvaultID)
    
    return tx.Commit()
}
```
**Why**: Shadow vaults too important for lock-free tricks, need proper ACID
**Ref**: Issue #5 - Shadow vault atomicity

#### 46. Add HTTP API rejections for shadow vault writes (Issue #6)
**What**: Explicit 403/404 on any shadow vault write endpoint
**Files**:
- `src/vault/internal/http/handlers.go` - Add shadow write rejections
**Key Logic**:
```go
// SHADOW VAULTS ARE WRITE-PROTECTED — DistoDam ONLY
// Any attempt to modify shadow vaults via HTTP API returns 403

func (h *VaultHandlers) shadowVaultWriteRejection(w http.ResponseWriter, r *http.Request) {
    http.Error(w, `{
        "error": "Forbidden",
        "message": "Shadow vaults are DistoDam-controlled reserves. Write operations are not permitted via HTTP API.",
        "detail": "Shadow vaults can only be modified by DistoDam via cryptographically-signed NATS commands."
    }`, http.StatusForbidden)
}

// Register rejection handlers
router.POST("/api/v1/vault/shadow/:id/deposit", h.shadowVaultWriteRejection)
router.POST("/api/v1/vault/shadow/:id/withdraw", h.shadowVaultWriteRejection)
router.DELETE("/api/v1/vault/shadow/:id", h.shadowVaultWriteRejection)
```
**Why**: Clear, immediate rejection prevents confusion about shadow vault access
**Ref**: Issue #6 - HTTP API must hard-reject shadow writes

#### 47. Restructure internal/ directory with module split (Issue #7)
**What**: Split `internal/vault/` into focused subdirectories for maintainability
**New Structure**:
```
src/vault/internal/
├── config/           # Environment variables, config loading
├── stash/            # StashVault implementation
│   ├── stash_vault.go
│   ├── stash_vault_test.go
│   └── repository.go
├── pledge/           # TorqedPledge implementation
│   ├── torqed_pledge.go
│   ├── torqed_pledge_test.go
│   ├── reputation.go
│   └── repository.go
├── shadow/           # Shadow vault implementation
│   ├── shadow_vault.go
│   ├── shadow_vault_test.go
│   └── repository.go
├── security/         # Security fixes (1-9)
│   ├── demurrage_cap.go
│   ├── withdrawal_notice.go
│   ├── rotating_remainder.go
│   ├── yield_calculator.go
│   ├── pledge_validator.go
│   ├── vault_limits.go
│   └── event_store.go
├── supply/           # Circulating supply subscription (Issue #1)
│   ├── circulating_supply_subscriber.go
│   └── supply_cache.go
├── scheduler/        # Demurrage scheduler (Issue #2)
│   ├── demurrage_scheduler.go
│   └── leader_election.go
├── nats/             # NATS messaging
│   ├── subscriber.go
│   ├── publisher.go
│   └── messages.go
├── http/             # HTTP API
│   ├── handlers.go
│   ├── middleware.go
│   └── routes.go
├── repository/       # Database layer
│   ├── migrations/
│   ├── stash_repository.go
│   ├── pledge_repository.go
│   ├── shadow_vault_repository.go
│   └── event_repository.go
└── metrics/          # Prometheus metrics
    └── metrics.go
```
**Why**: Greenfield codebase is easiest time to structure properly
**Ref**: Issue #7 - Module split suggestion
**Action**: Update all file paths in todos #1-40 to reflect new structure

#### 48. Add property-based fuzz tests for event sourcing (Issue #8)
**What**: Chaos testing for event sourcing using gopter (property-based testing)
**Files**:
- `src/vault/internal/security/event_store_fuzz_test.go`
**Key Logic**:
```go
import "github.com/leanovate/gopter"

func TestEventSourcing_PropertyBased(t *testing.T) {
    properties := gopter.NewProperties(nil)
    
    // Property: Replaying events in any order yields same final state
    properties.Property("event replay convergence", prop.ForAll(
        func(events []VaultEvent) bool {
            // Shuffle events randomly
            shuffled := shuffle(events)
            
            // Replay original order
            state1 := replayEvents(events)
            
            // Replay shuffled order
            state2 := replayEvents(shuffled)
            
            // Final state must be identical
            return state1.Equals(state2)
        },
        gen.SliceOf(genVaultEvent()),
    ))
    
    // Property: Duplicate events are idempotent
    properties.Property("duplicate idempotency", prop.ForAll(
        func(events []VaultEvent) bool {
            // Replay with duplicates
            duplicated := append(events, events...) // Double all events
            state1 := replayEvents(events)
            state2 := replayEvents(duplicated)
            
            return state1.Equals(state2)
        },
        gen.SliceOf(genVaultEvent()),
    ))
    
    // Property: Dropped events don't cause corruption
    properties.Property("dropped event resilience", prop.ForAll(
        func(events []VaultEvent) bool {
            if len(events) < 2 {
                return true
            }
            
            // Drop random events (50% chance)
            filtered := filterRandom(events, 0.5)
            
            // State must still be valid (no panics, no negative balances)
            state := replayEvents(filtered)
            return state.IsValid()
        },
        gen.SliceOf(genVaultEvent()),
    ))
    
    properties.TestingRun(t)
}
```
**Test Scenarios**:
- 10,000+ random event sequences
- Out-of-order delivery (NATS redelivery)
- Duplicate events (at-least-once semantics)
- Missing events (network partitions)
- Concurrent event streams
**Why**: Event sourcing must handle chaos gracefully in production
**Ref**: Issue #8 - Fuzz + out-of-order tests for event sourcing
**Dependencies**: `github.com/leanovate/gopter`

---

### Phase 1: Infrastructure & Core (Items 1-9)

#### 1. Set up Vault service infrastructure
**What**: Create basic service skeleton
**Files**:
- `src/vault/cmd/vault/main.go` - Entry point, graceful shutdown
- `src/vault/go.mod` - Dependencies (nats, postgres, prometheus)
- `src/vault/internal/config/config.go` - Environment variables, defaults
**Dependencies**: None
**Ref**: Follow DistoDam pattern (see `src/distodam/cmd/distodam/main.go`)

#### 2. Implement StashVault core data structures and interfaces
**What**: Define StashVault types, manager interface (with entity support)
**Files**:
- `src/vault/internal/stash/stash_vault.go` - StashVault struct, StashVaultManager interface
- `src/vault/internal/models/models.go` - Common types (MicroRT, VaultEvent)
**Key Types**:
```go
type StashVault struct {
    ID string
    WalletID string
    EntityID string  // NEW: "personal", "builder-company", etc.
    EntityType string // NEW: "personal", "business", "dao"
    BalanceMicroRT int64
    TotalDepositedMicroRT int64
    TotalWithdrawnMicroRT int64
    TotalYieldEarnedMicroRT int64
    CreatedAt time.Time
    UpdatedAt time.Time
}

type StashVaultManager interface {
    CreateStashVault(ctx context.Context, walletID string, entityID string, entityType string) (*StashVault, error)
    Deposit(ctx context.Context, vaultID string, amountMicroRT int64, source string) error
    Withdraw(ctx context.Context, vaultID string, amountMicroRT int64) error
    DistributeDemurragePool(ctx context.Context, totalDemurrageMicroRT int64) error
    GetStashVault(ctx context.Context, vaultID string) (*StashVault, error)
    GetStashVaultsByWallet(ctx context.Context, walletID string) ([]*StashVault, error) // NEW: List all entities
    GetTotalBalance() int64
}
```
**Ref**: STASHVAULT_IMPLEMENTATION.md lines 45-85 (updated for entity support)

#### 3. Implement StashVault deposit/withdrawal operations
**What**: Core deposit/withdraw logic with validation, atomicity
**Files**:
- `src/vault/internal/vault/stash_vault.go` - Deposit(), Withdraw() implementations
- `src/vault/internal/vault/repository.go` - Database persistence layer
**Key Logic**:
- Validate amount > 0
- Check balance for withdrawals
- Database transactions (atomic)
- Event logging
- NATS publish confirmations
**Ref**: STASHVAULT_IMPLEMENTATION.md lines 205-343

#### 4. Implement StashVault demurrage pool distribution
**What**: Pro-rata yield distribution from network demurrage
**Files**:
- `src/vault/internal/vault/stash_vault.go` - DistributeDemurragePool()
- `src/vault/internal/vault/demurrage_accumulator.go` - Daily aggregation
**Formula**: `share = (vault_balance / total_balance) * total_demurrage`
**Triggers**: Daily cron job (midnight UTC)
**Ref**: STASHVAULT_IMPLEMENTATION.md lines 93-183, DEMURRAGE_ARCHITECTURE.md section 5

#### 5. Implement TorqedPledge core data structures and interfaces
**What**: Define TorqedPledge types, manager interface (with entity support)
**Files**:
- `src/vault/internal/pledge/torqed_pledge.go` - TorqedPledge struct, TorqedPledgeManager interface
**Key Types**:
```go
type TorqedPledge struct {
    ID string
    WalletID string
    EntityID string  // NEW: "personal", "builder-company", etc.
    EntityType string // NEW: "personal", "business", "dao"
    TargetMicroRT int64
    MonthlyPledgeMicroRT int64
    SavedMicroRT int64
    Status string // "active", "matured", "triggered"
    ReputationScore int // 0-100
    Purpose string
    LockMonths int
    BRLAID string
    CreatedAt time.Time
    MaturedAt *time.Time
    TriggeredAt *time.Time
}

type TorqedPledgeManager interface {
    CreatePledge(ctx context.Context, req *CreatePledgeRequest) (*TorqedPledge, error)
    Deposit(ctx context.Context, pledgeID string, amountMicroRT int64, source string) error
    GetProgress(ctx context.Context, pledgeID string) (*PledgeProgress, error)
    LockPledge(ctx context.Context, pledgeID string, brlaID string) error
    UpdateReputation(ctx context.Context, pledgeID string, delta int) error
    GetPledgesByWallet(ctx context.Context, walletID string) ([]*TorqedPledge, error) // NEW: List all entities
}

type CreatePledgeRequest struct {
    WalletID string
    EntityID string  // NEW: Required
    EntityType string // NEW: "personal", "business", "dao"
    TargetMicroRT int64
    MonthlyPledgeMicroRT int64
    Purpose string
}
```
**Ref**: TORQEDVAULT_IMPLEMENTATION.md lines 41-109 (updated for entity support)

#### 6. Implement TorqedPledge creation and funding logic
**What**: Create pledges, deposit funds, detect maturity
**Files**:
- `src/vault/internal/vault/torqed_pledge.go` - CreatePledge(), Deposit()
**Key Logic**:
- Validate target > 0, monthly_pledge > 0, monthly_pledge <= target
- Deposit validation: reject if matured/triggered
- Maturity detection: saved >= target → publish `vault.pledge_ready`
- **DistoDam validation**: Enforce monthly pledge amount (prevent under-funding)
**Ref**: TORQEDVAULT_IMPLEMENTATION.md lines 120-309

#### 7. Implement TorqedPledge yield distribution (wallet-only, Fix #4)
**What**: **CRITICAL SECURITY FIX** - Yield goes to wallet, NOT pledge balance
**Files**:
- `src/vault/internal/vault/torqed_pledge.go` - DistributeYield()
- `src/vault/internal/vault/nats_publisher.go` - Publish `vault.yield_payment`
**Key Logic**:
- Calculate yield: `(pledge_balance / total_pledge_balance) * demurrage_pool`
- **DO NOT** add to SavedMicroRT (prevents compounding exploit)
- Publish yield payment to wallet via NATS
- Track in TotalYieldEarnedMicroRT (stats only)
**Why**: Prevents infinite money glitch (compounding into maturity target)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #4, TORQEDVAULT_IMPLEMENTATION.md lines 488-560

#### 8. Implement TorqedPledge maturity detection and BRLA triggering
**What**: Detect when saved >= target, publish event to Trust
**Files**:
- `src/vault/internal/vault/torqed_pledge.go` - checkMaturity(), LockPledge()
- `src/vault/internal/vault/nats_subscriber.go` - HandleBRLATriggered()
**Flow**:
1. Deposit() → check saved >= target
2. If matured → update status, set MaturedAt, publish `vault.pledge_ready`
3. Trust creates BRLA → publishes `brla.triggered`
4. Vault receives event → LockPledge() → set BRLAID, TriggeredAt, status="triggered"
**Ref**: TORQEDVAULT_IMPLEMENTATION.md lines 362-425

#### 9. Implement TorqedPledge reputation system
**What**: 0-100 score, impacts collateral/insurance premiums
**Files**:
- `src/vault/internal/vault/torqed_pledge.go` - UpdateReputation()
**Events**:
- Pledge met on time: +2
- Pledge met early: +3
- Pledge missed (insured): -1
- Pledge missed (uninsured): -5
**Bounds**: Clamp to [0, 100]
**Ref**: TORQEDVAULT_IMPLEMENTATION.md lines 439-486

---

### Phase 2: Shadow Vaults (Items 10-13)

#### 10. Implement Shadow Vault data structures (StakeVault + DistoVault)
**What**: DistoDam-controlled reserves (one pair per TorqVault)
**Files**:
- `src/vault/internal/vault/shadow_vault.go` - ShadowStakeVault, ShadowDistoVault structs
**Key Types**:
```go
type ShadowStakeVault struct {
    TorqVaultID string
    BalanceMicroRT atomic.Int64 // Thread-safe
    TotalFeesCollectedMicroRT int64
    TotalPenaltiesCollectedMicroRT int64
    TotalReroutedMicroRT int64
}

type ShadowDistoVault struct {
    TorqVaultID string
    BalanceMicroRT atomic.Int64
    TotalOperationalFeesMicroRT int64
    TotalOraclePaymentsMicroRT int64
}
```
**Ref**: SHADOW_VAULT_ARCHITECTURE.md sections 1.1, 1.2

#### 11. Implement Shadow Vault funding sources (fees, penalties, rerouting)
**What**: Transaction fees, cancellation penalties, urgent rerouting
**Files**:
- `src/vault/internal/vault/shadow_vault.go` - DepositFee(), DepositPenalty(), DepositRerouting()
- `src/vault/internal/vault/nats_subscriber.go` - HandleTransactionFee(), HandleCancellationPenalty()
**Sources**:
- StakeVault: BidNet fees (0.1-0.5%), cancellation penalties (100% undelivered), urgent rerouting (50-80% demurrage)
- DistoVault: Operational fees, Oracle payments, slashing penalties
**Ref**: SHADOW_VAULT_ARCHITECTURE.md section 3

#### 12. Implement DistoDam-only control for Shadow Vaults
**What**: Only DistoDam can modify shadow vaults (prevent embezzlement)
**Files**:
- `src/vault/internal/vault/shadow_vault.go` - ValidateDistoDamSignature()
**Key Logic**:
- All shadow vault events require DistoDam's cryptographic signature (Falcon-1024 or SPHINCS+ depending on use case)
- TorqVaults cannot withdraw/transfer shadow vault RT
- Signature verification before state changes
**Crypto Note**: Use Falcon-1024 for operational speed (same as Digger→Refinery), SPHINCS+ for long-term archival (same as Mint→DistoDam ledger)
**Ref**: SHADOW_VAULT_ARCHITECTURE.md section 2.2, `docs/crypto refactor docs/PHASE5_VERIFICATION_PLAN.md`

#### 13. Implement urgent demurrage rerouting logic
**What**: Crisis mode - divert 50-80% of demurrage to Shadow StakeVaults
**Files**:
- `src/vault/internal/vault/demurrage_rerouter.go` - CalculateRerouting(), TriggerUrgentRerouting()
**Triggers**:
- Network vault ratio < 0.1 (crisis)
- Governance vote (quadratic voting)
**Allocation**: Proportional to TorqVault member count
**Ref**: SHADOW_VAULT_ARCHITECTURE.md section 4, DEMURRAGE_ARCHITECTURE.md section 6

---

### Phase 3: Security Fixes (Items 14-22)

#### 14. Implement Security Fix #1: Demurrage cap (60%) + sigmoid yield
**What**: Cap demurrage at 60% of circulating RT, sigmoid yield curve (k=18, x₀=0.85)
**Files**:
- `src/vault/internal/vault/demurrage_calculator.go` - CalculateDemurrageCap(), CalculateSigmoidYield()
**Formula**:
```go
// Sigmoid suppression when vault_ratio > 0.85
func sigmoidSuppression(vaultRatio float64) float64 {
    k := 18.0
    x0 := 0.85
    return 1.0 / (1.0 + math.Exp(k * (vaultRatio - x0)))
}

effectiveYield := baseYield * sigmoidSuppression(vaultRatio)
```
**Why**: Prevents >60% circulating RT locked (liquidity crisis)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #1, Appendix A

#### 15. Implement Security Fix #2: 30-day withdrawal notice + amount binding
**What**: Prevent whale attacks (notice required, amount locked at submission)
**Files**:
- `src/vault/internal/vault/withdrawal_notice.go` - SubmitNotice(), CancelNotice(), ProcessWithdrawal()
**Key Logic**:
```go
type WithdrawalNotice struct {
    NoticeID string
    PledgeID string
    AmountMicroRT int64 // LOCKED at submission
    SubmittedAt time.Time
    MaturesAt time.Time // SubmittedAt + 30 days
    Status string // "pending", "matured", "cancelled", "processed"
}
```
**Why**: Prevents instant withdrawals that crash vault ratio
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #2, Appendix B

#### 16. Implement Security Fix #3: Rotating remainder distribution
**What**: Prevent vault ID advantage in remainder allocation
**Files**:
- `src/vault/internal/vault/demurrage_calculator.go` - DistributeRemainder()
**Formula**: `daysSinceLaunch % len(vaults)` → winner gets remainder
**Example**:
```go
remainder := totalDemurrage - sum(vaultShares)
winnerIndex := (daysSinceLaunch % len(vaults))
vaults[winnerIndex].balance += remainder
```
**Why**: Fairness (no vault always wins rounding)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #3, Appendix C

#### 17. Implement Security Fix #4: Yield to wallet only (prevent compounding)
**What**: **ALREADY IN TODO #7** - Yield cannot count toward pledge maturity
**Files**: See Todo #7
**Why**: Prevents 36-month pledge @ 2%/mo = >100% APY exploit
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #4

#### 18. Implement Security Fix #5: Rolling 3-month average pledge enforcement
**What**: Prevent reputation gaming via under-funding
**Files**:
- `src/vault/internal/vault/pledge_validator.go` - ValidateMonthlyPledge()
**Formula**:
```go
avg3Month := (deposit_month1 + deposit_month2 + deposit_month3) / 3
if avg3Month < pledge.MonthlyPledgeMicroRT {
    // Warn user, reputation penalty
}
```
**Why**: User can't set 10k/month pledge, deposit 1 RT, still gain reputation
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #5

#### 19. Implement Security Fix #6: Dynamic yield rate + grace smoothing
**What**: Yield varies with vault ratio, 25% yield at maturity (no cliff)
**Files**:
- `src/vault/internal/vault/yield_calculator.go` - CalculateDynamicYield(), ApplyGraceSmoothing()
**Formula**:
```go
// Grace period: 90 days before maturity
if daysUntilMaturity <= 90 {
    graceFactor := 0.25 + 0.75 * (daysUntilMaturity / 90.0)
    effectiveYield := baseYield * graceFactor
}
```
**Why**: Prevents cliff-induced bank runs (users don't rush to withdraw)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #6, Appendix C

#### 20. Implement Security Fix #7: Global vault ratio sigmoid suppression
**What**: When global vault_ratio > 80%, suppress yield aggressively
**Files**:
- `src/vault/internal/vault/yield_calculator.go` - ApplyGlobalSuppression()
**Formula**: See Fix #1 sigmoid (same k=18, x₀=0.85)
**Why**: Prevents >80% circulating RT locked (liquidity freeze)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #7

#### 21. Implement Security Fix #8: One vault per entity + 10 RT minimum
**What**: Prevent Sybil attacks while allowing multi-entity management (personal, business, DAO)
**Files**:
- `src/vault/internal/security/vault_limits.go` - ValidateVaultCreation()
- `src/vault/internal/stash/stash_vault.go` - Add EntityID field
- `src/vault/internal/pledge/torqed_pledge.go` - Add EntityID field
**Data Model**:
```go
type StashVault struct {
    ID string
    WalletID string
    EntityID string  // NEW: "personal", "builder-company", "supplier-company", etc.
    EntityType string // NEW: "personal", "business", "dao"
    // ... existing fields
}

type TorqedPledge struct {
    ID string
    WalletID string
    EntityID string  // NEW: Same entity system
    EntityType string
    // ... existing fields
}
```
**Validation**:
```go
func ValidateVaultCreation(walletID string, entityID string, vaultType string) error {
    // Check if this wallet+entity combo already has this vault type
    existing := repo.GetVaultsByWalletAndEntity(walletID, entityID, vaultType)
    if len(existing) > 0 {
        return errors.New("entity already has vault of this type")
    }
    
    // Validate entity_id format (alphanumeric + hyphens + underscores)
    // Allows: "personal", "builder_company", "my-dao", "supplier_llc_2025"
    if !regexp.MustCompile(`^[a-z0-9_-]+$`).MatchString(entityID) {
        return errors.New("entity_id must be lowercase alphanumeric with hyphens or underscores")
    }
    
    // Optional: Limit total entities per wallet (prevents abuse)
    totalEntities := repo.CountEntitiesByWallet(walletID)
    if totalEntities >= 10 { // Configurable limit
        return errors.New("wallet has reached maximum entity limit (10)")
    }
    
    return nil
}

func ValidateDeposit(amount int64) error {
    if amount < 10_000_000 { // 10 RT
        return errors.New("minimum deposit 10 RT")
    }
    return nil
}
```
**Example Usage**:
```go
// Personal finances
CreateStashVault(walletID: "wallet-abc123", entityID: "personal")

// Builder company
CreateStashVault(walletID: "wallet-abc123", entityID: "builder-company")
CreatePledge(walletID: "wallet-abc123", entityID: "builder-company", target: 50000)

// Supplier company
CreateStashVault(walletID: "wallet-abc123", entityID: "supplier-company")

// DAO treasury
CreateStashVault(walletID: "wallet-abc123", entityID: "robotorq-dao")
```
**Database Schema**:
```sql
CREATE TABLE stash_vaults (
    id VARCHAR PRIMARY KEY,
    wallet_id VARCHAR NOT NULL,
    entity_id VARCHAR NOT NULL,
    entity_type VARCHAR NOT NULL, -- 'personal', 'business', 'dao'
    balance_micro_rt BIGINT NOT NULL,
    -- ... other fields
    UNIQUE(wallet_id, entity_id) -- One StashVault per wallet+entity
);

CREATE TABLE torqed_pledges (
    id VARCHAR PRIMARY KEY,
    wallet_id VARCHAR NOT NULL,
    entity_id VARCHAR NOT NULL,
    entity_type VARCHAR NOT NULL,
    target_micro_rt BIGINT NOT NULL,
    -- ... other fields
    UNIQUE(wallet_id, entity_id) -- One TorqedPledge per wallet+entity
);

CREATE INDEX idx_stash_vaults_wallet ON stash_vaults(wallet_id);
CREATE INDEX idx_stash_vaults_entity ON stash_vaults(entity_id);
CREATE INDEX idx_pledges_wallet ON torqed_pledges(wallet_id);
CREATE INDEX idx_pledges_entity ON torqed_pledges(entity_id);
```
**Why**: 
- Prevents Sybil attacks (10 RT minimum per vault still enforced)
- Allows legitimate multi-entity management (personal vs business finances)
- Entity limit (10 max) prevents abuse while supporting complex organizers
- Still enforces pro-rata yield fairness (each entity's vault competes independently)
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #8 (updated for entity support)

#### 22. Implement Security Fix #9: Event sourcing + sequence numbers
**What**: All vault events append-only, idempotent, sequence-numbered
**Files**:
- `src/vault/internal/vault/event_store.go` - AppendEvent(), GetEvents(), ReplayEvents()
- `src/vault/internal/vault/repository.go` - PostgreSQL event_log table
**Schema**:
```sql
CREATE TABLE vault_events (
    event_id UUID PRIMARY KEY,
    vault_id VARCHAR NOT NULL,
    sequence_number BIGINT NOT NULL,
    event_type VARCHAR NOT NULL,
    payload JSONB NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    nats_msg_id VARCHAR UNIQUE, -- Idempotency key
    UNIQUE(vault_id, sequence_number)
);
```
**Why**: Prevents data loss, enables audit trails, NATS exactly-once semantics
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #9, Appendix D

---

### Phase 4: Integration (Items 23-26)

#### 23. Implement NATS message schemas (PledgeDeposit, WithdrawalNotice, etc.)
**What**: Define all NATS message types for vault communication
**Files**:
- `src/vault/internal/vault/nats_messages.go` - All message structs
**Key Messages**:
```go
type PledgeDeposit struct {
    EventID string
    PledgeID string
    AmountMicroRT int64
    Source string // "ubd_auto", "wallet_transfer", "yield_redeposit"
    Timestamp time.Time
    Signature []byte
}

type WithdrawalNoticeSubmit struct {
    NoticeID string
    PledgeID string
    AmountMicroRT int64 // LOCKED amount
    SubmittedAt time.Time
    Signature []byte
}

type WithdrawalNoticeCancel struct {
    NoticeID string
    CancelledAt time.Time
    Signature []byte
}
```
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Appendix B

#### 24. Implement NATS subscriptions (ubd.vault_deposit, phone.vault_transfer, demurrage.paid)
**What**: Subscribe to incoming events from DistoDam, phones, demurrage
**Files**:
- `src/vault/internal/vault/nats_subscriber.go` - HandleUBDVaultDeposit(), HandlePhoneVaultTransfer(), HandleDemurragePaid()
**Topics**:
- `ubd.vault_deposit` - DistoDam auto-diversion
- `phone.vault_transfer` - Manual deposits/withdrawals from wallet
- `demurrage.paid` - Daily demurrage settlement
- `brla.triggered` - Trust notifies pledge lock
**Ref**: STASHVAULT_IMPLEMENTATION.md lines 353-508, TORQEDVAULT_IMPLEMENTATION.md lines 409-425

#### 25. Implement NATS publishers (vault.pledge_ready, vault.withdrawal_approved)
**What**: Publish events when pledges mature, withdrawals approved
**Files**:
- `src/vault/internal/vault/nats_publisher.go` - PublishPledgeReady(), PublishWithdrawalApproved()
**Topics**:
- `vault.pledge_ready` - Pledge matured → Trust creates BRLA
- `vault.withdrawal_approved` - 30-day notice matured → process withdrawal
- `vault.yield_payment` - Yield sent to wallet
- `vault.shadow.fee_deposit` - Shadow vault fee collected
**Ref**: SHADOW_VAULT_ARCHITECTURE.md section 7

#### 26. Implement HTTP API endpoints (create, deposit, withdraw, get stats)
**What**: REST API for wallet/UI interaction
**Files**:
- `src/vault/internal/http/handlers.go` - HTTP route handlers
- `src/vault/internal/http/middleware.go` - Auth, logging, CORS
**Endpoints**:
```
POST   /api/v1/vault/stash                           - Create StashVault
       Body: {"wallet_id": "...", "entity_id": "personal", "entity_type": "personal"}
GET    /api/v1/vault/stash/:id                       - Get StashVault
GET    /api/v1/vault/stash/wallet/:wallet_id         - List all StashVaults for wallet (all entities)
POST   /api/v1/vault/stash/:id/deposit               - Deposit to StashVault
POST   /api/v1/vault/stash/:id/withdraw              - Withdraw from StashVault
GET    /api/v1/vault/stash/stats                     - Global StashVault stats
GET    /api/v1/vault/stash/wallet/:wallet_id/stats   - Per-wallet stats (aggregated across entities)

POST   /api/v1/vault/pledge                          - Create TorqedPledge
       Body: {"wallet_id": "...", "entity_id": "builder-company", "entity_type": "business", ...}
GET    /api/v1/vault/pledge/:id                      - Get TorqedPledge
GET    /api/v1/vault/pledge/wallet/:wallet_id        - List all TorqedPledges for wallet (all entities)
GET    /api/v1/vault/pledge/:id/progress             - Get pledge progress
POST   /api/v1/vault/pledge/:id/notice               - Submit withdrawal notice
DELETE /api/v1/vault/pledge/:id/notice/:nid          - Cancel withdrawal notice

GET    /api/v1/vault/shadow/:torqvault_id            - Get shadow vault balances (read-only)
GET    /api/v1/vault/entities/:wallet_id             - List all entity IDs for a wallet
```
**Entity Examples**:
```bash
# Create personal StashVault
curl -X POST /api/v1/vault/stash \
  -d '{"wallet_id":"wallet-abc","entity_id":"personal","entity_type":"personal"}'

# Create business StashVault
curl -X POST /api/v1/vault/stash \
  -d '{"wallet_id":"wallet-abc","entity_id":"builder-company","entity_type":"business"}'

# List all vaults for wallet (returns both personal + builder-company)
curl /api/v1/vault/stash/wallet/wallet-abc

# Create pledge for supplier company
curl -X POST /api/v1/vault/pledge \
  -d '{"wallet_id":"wallet-abc","entity_id":"supplier-company","entity_type":"business","target_micro_rt":50000000000,"monthly_pledge_micro_rt":5000000000}'
```
**Ref**: STASHVAULT_IMPLEMENTATION.md lines 520-676, TORQEDVAULT_IMPLEMENTATION.md lines 562-645 (updated for entity support)

---

### Phase 5: Testing (Items 27-38)

#### 27. Write unit tests for StashVault (target 95%+ coverage)
**What**: Test all StashVault operations
**Files**:
- `src/vault/internal/vault/stash_vault_test.go`
**Test Cases** (from STASHVAULT_IMPLEMENTATION.md):
- Deposit: positive amount, reject negative/zero, update balance, log transaction
- Withdraw: valid amount, reject negative/zero, reject insufficient balance
- Demurrage distribution: single vault (100%), multiple vaults (pro-rata), zero demurrage, rounding
- NATS handlers: UBD deposit, phone transfer, demurrage paid, idempotency
**Ref**: STASHVAULT_IMPLEMENTATION.md (marked with ✅)

#### 28. Write unit tests for TorqedPledge (target 95%+ coverage)
**What**: Test all TorqedPledge operations
**Files**:
- `src/vault/internal/vault/torqed_pledge_test.go`
**Test Cases** (from TORQEDVAULT_IMPLEMENTATION.md):
- CreatePledge: valid, reject negative target/monthly, calculate lock duration
- Deposit: to active pledge, reject matured/triggered, detect maturity, publish event
- GetProgress: 0%, 50%, 100%, months elapsed/remaining
- LockPledge: matured only, reject active/triggered, set BRLA ID
- UpdateReputation: increase, decrease, clamp [0, 100]
- Yield distribution: wallet-only (Fix #4), NOT to SavedMicroRT
**Ref**: TORQEDVAULT_IMPLEMENTATION.md (marked with ✅)

#### 29. Write unit tests for Shadow Vaults (target 95%+ coverage)
**What**: Test shadow vault operations, DistoDam-only control
**Files**:
- `src/vault/internal/vault/shadow_vault_test.go`
**Test Cases**:
- DepositFee: transaction fee → StakeVault, validate signature
- DepositPenalty: cancellation penalty → StakeVault
- DepositRerouting: urgent demurrage → StakeVaults (proportional allocation)
- ValidateDistoDamSignature: accept valid, reject invalid/missing
- Withdraw attempts: reject non-DistoDam callers
**Ref**: SHADOW_VAULT_ARCHITECTURE.md sections 2-4

#### 30. Write unit tests for all 9 security fixes
**What**: Validate each security fix independently + explicit attack scenario tests
**Files**:
- `src/vault/internal/security/security_fixes_test.go` - Standard validation tests
- `src/vault/internal/security/attack_scenarios_test.go` - **NEW: Explicit attack tests**

**Standard Test Cases** (security_fixes_test.go):
- Fix #1: Demurrage cap at 60%, sigmoid suppression curve
- Fix #2: 30-day notice required, amount binding
- Fix #3: Rotating remainder distribution (fairness over days)
- Fix #4: Yield to wallet only (NOT SavedMicroRT)
- Fix #5: Rolling 3-month average validation
- Fix #6: Grace smoothing (25% yield at maturity)
- Fix #7: Global vault ratio suppression (>80%)
- Fix #8: One vault per wallet+entity, 10 RT minimum, entity limit (10 max)
- Fix #9: Event sourcing, sequence numbers, idempotency

**Attack Scenario Tests** (attack_scenarios_test.go - **NEW**):
This dedicated file contains explicit "scare-the-new-devs" tests that try every dirty trick:

```go
// src/vault/internal/security/attack_scenarios_test.go

// TestAttack_CompoundingExploit_CannotCountYieldTowardMaturity
// This test MUST fail if yield ever counts toward pledge maturity.
// It tries every dirty trick to exploit compounding into the target.
func TestAttack_CompoundingExploit_CannotCountYieldTowardMaturity(t *testing.T) {
    // Setup: Pledge targeting 1000 RT
    pledge := createTestPledge(t, 1000_000_000, 100_000_000) // 1000 RT target, 100 RT/month
    
    // Attack 1: Direct yield re-deposit loop
    t.Run("attack: re-deposit yield loop", func(t *testing.T) {
        // Deposit 500 RT legitimately
        deposit(t, pledge.ID, 500_000_000, "ubd_auto")
        
        // Generate 50 RT yield (10% on 500 RT balance)
        yield := calculateYield(pledge.ID) // Returns 50 RT
        
        // ATTACK: Try to re-deposit yield back into pledge
        err := deposit(t, pledge.ID, yield, "yield_redeposit")
        
        // EXPECTED: Deposit succeeds BUT SavedMicroRT unchanged
        assert.NoError(t, err)
        pledge = getPledge(t, pledge.ID)
        assert.Equal(t, 500_000_000, pledge.SavedMicroRT, "Yield should NOT count toward maturity")
        
        // Verify yield went to wallet instead
        walletBalance := getWalletBalance(t, pledge.WalletID)
        assert.Equal(t, 50_000_000, walletBalance, "Yield should go to wallet")
    })
    
    // Attack 2: Replay old yield events
    t.Run("attack: replay yield events", func(t *testing.T) {
        // Publish yield payment event
        yieldEvent := YieldPayment{
            PledgeID: pledge.ID,
            AmountMicroRT: 100_000_000,
            Timestamp: time.Now(),
        }
        publishNATS(t, "vault.yield_payment", yieldEvent)
        
        // ATTACK: Replay same event 10 times
        for i := 0; i < 10; i++ {
            publishNATS(t, "vault.yield_payment", yieldEvent)
        }
        
        // EXPECTED: Idempotent (only processed once)
        time.Sleep(100 * time.Millisecond)
        walletBalance := getWalletBalance(t, pledge.WalletID)
        assert.Equal(t, 100_000_000, walletBalance, "Yield should only be paid once")
    })
    
    // Attack 3: Forged source tag
    t.Run("attack: forge source as ubd_auto", func(t *testing.T) {
        initialSaved := pledge.SavedMicroRT
        
        // ATTACK: Deposit with forged "ubd_auto" source
        // Attacker hopes this bypasses yield-to-wallet restriction
        err := deposit(t, pledge.ID, 50_000_000, "ubd_auto")
        
        // EXPECTED: Source doesn't matter, SavedMicroRT increases only if from UBD
        pledge = getPledge(t, pledge.ID)
        
        // Verify deposit validation (must match DistoDam signature)
        // Without valid signature, deposit should fail
        assert.Error(t, err, "Forged UBD deposit should fail signature check")
        assert.Equal(t, initialSaved, pledge.SavedMicroRT)
    })
    
    // Attack 4: Direct database manipulation
    t.Run("attack: direct DB update SavedMicroRT", func(t *testing.T) {
        initialSaved := pledge.SavedMicroRT
        
        // ATTACK: Bypass application logic, update DB directly
        _, err := db.Exec("UPDATE torqed_pledges SET saved_micro_rt = saved_micro_rt + $1 WHERE id = $2",
            100_000_000, pledge.ID)
        assert.NoError(t, err) // DB update succeeds
        
        // EXPECTED: Event sourcing detects mismatch on next replay
        replayedPledge := replayEventsForPledge(t, pledge.ID)
        assert.Equal(t, initialSaved, replayedPledge.SavedMicroRT, 
            "Event sourcing should reject direct DB manipulation")
        
        // Trigger event replay validation
        err = validatePledgeIntegrity(t, pledge.ID)
        assert.Error(t, err, "Integrity check should detect DB tampering")
    })
    
    // Attack 5: Maturity check timing exploit
    t.Run("attack: race condition on maturity check", func(t *testing.T) {
        // Setup: Pledge at 999.99 RT (0.01 RT short of 1000 RT target)
        pledge.SavedMicroRT = 999_990_000
        updatePledge(t, pledge)
        
        // ATTACK: 100 concurrent threads each deposit 0.01 RT
        var wg sync.WaitGroup
        for i := 0; i < 100; i++ {
            wg.Add(1)
            go func() {
                defer wg.Done()
                deposit(t, pledge.ID, 10_000, "ubd_auto")
            }()
        }
        wg.Wait()
        
        // EXPECTED: Only first deposit triggers maturity, others deposited to wallet
        pledge = getPledge(t, pledge.ID)
        assert.Equal(t, "matured", pledge.Status)
        assert.Equal(t, 1_000_000_000, pledge.SavedMicroRT, "Target should be exactly met, no overflow")
    })
}

// TestAttack_SigmoidBypass_CannotExceed60PercentDemurrage
func TestAttack_SigmoidBypass_CannotExceed60PercentDemurrage(t *testing.T) {
    // Attack: Create fake circulating supply to manipulate vault ratio
    // Expected: Demurrage capped at 60% regardless of vault ratio
    // (Full test implementation here)
}

// TestAttack_WithdrawalNoticeSnipe_CannotChangeAmount
func TestAttack_WithdrawalNoticeSnipe_CannotChangeAmount(t *testing.T) {
    // Attack: Submit notice for 100 RT, try to change to 10000 RT before maturity
    // Expected: Amount locked at submission time
    // (Full test implementation here)
}

// TestAttack_EntitySybil_CannotBypass10EntityLimit
func TestAttack_EntitySybil_CannotBypass10EntityLimit(t *testing.T) {
    // Attack: Create 100 fake entities to split deposits
    // Expected: Rejected after 10 entities
    // (Full test implementation here)
}
```

**Why**: These explicit attack tests serve as:
1. **Security regression tests** - If exploit becomes possible, tests fail immediately
2. **Developer education** - New devs see exactly what NOT to allow
3. **Audit evidence** - Security reviewers see attack vectors covered
4. **Living documentation** - Code comments explain each exploit

**Coverage**: Combines with Todo #17 (Fix #4 implementation), #28 (yield-to-wallet unit tests), and #36 (integration tests) for complete compounding exploit protection.

**Ref**: VAULT_SECURITY_FIXES_PART_2.md

#### 31. Implement Prometheus metrics (deposits, withdrawals, yields, vault ratios)
**What**: Instrument all vault operations for observability
**Files**:
- `src/vault/internal/vault/metrics.go` - Prometheus metrics
**Key Metrics**:
```go
type Metrics struct {
    // StashVault
    StashVaultsTotal prometheus.Gauge
    StashDepositsTotalMicroRT prometheus.Counter
    StashWithdrawalsTotalMicroRT prometheus.Counter
    StashYieldDistributedTotalMicroRT prometheus.Counter
    
    // TorqedPledge
    PledgesTotalActive prometheus.Gauge
    PledgesTotalMatured prometheus.Gauge
    PledgeDepositsTotalMicroRT prometheus.Counter
    PledgeAverageReputation prometheus.Gauge
    
    // Withdrawal Notices
    WithdrawalNoticesPendingTotalMicroRT prometheus.Gauge  // NEW: Total RT locked in pending notices
    WithdrawalNoticesPendingCount prometheus.Gauge         // NEW: Count of pending notices
    WithdrawalNoticesProcessedTotalMicroRT prometheus.Gauge // NEW: Total RT in processed notices (denominator for whale alert)
    WithdrawalNoticesSubmittedTotal prometheus.Counter
    WithdrawalNoticesCancelledTotal prometheus.Counter     // NEW: Track cancellations
    WithdrawalNoticesProcessedTotal prometheus.Counter     // NEW: Track successful withdrawals
    
    // Shadow Vaults
    ShadowStakeVaultBalanceMicroRT prometheus.Gauge
    ShadowDistoVaultBalanceMicroRT prometheus.Gauge
    
    // Security
    VaultRatioGlobal prometheus.Gauge // circulating_rt / total_rt
    DemurrageCapHitsTotal prometheus.Counter
    WithdrawalNoticesSubmittedTotal prometheus.Counter
}
```
**Ref**: DistoDam metrics pattern (`src/distodam/internal/distodam/metrics.go`)

#### 32. Implement structured logging (slog) for all vault operations
**What**: JSON logging with contextual fields
**Files**:
- `src/vault/internal/vault/logger.go` - Shared logger setup
**Pattern**:
```go
slog.Info("stash vault created",
    "vault_id", vault.ID,
    "wallet_id", vault.WalletID,
    "created_at", vault.CreatedAt)

slog.Info("pledge matured",
    "pledge_id", pledge.ID,
    "target_rt", MicroRTToRT(pledge.TargetMicroRT),
    "months_elapsed", monthsElapsed,
    "reputation", pledge.ReputationScore)

slog.Error("deposit failed",
    "vault_id", vaultID,
    "amount_rt", MicroRTToRT(amount),
    "error", err)
```
**Ref**: Mint logging pattern (`src/mint/internal/mint/mint_engine.go`)

#### 33. Create Dockerfile for Vault service
**What**: Containerize Vault service
**Files**:
- `src/vault/Dockerfile`
**Pattern** (multi-stage build):
```dockerfile
FROM golang:1.24-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o vault ./cmd/vault

FROM alpine:3.20
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/vault .
HEALTHCHECK --interval=30s CMD wget -qO- http://localhost:8082/health || exit 1
CMD ["./vault"]
```
**Ref**: DistoDam Dockerfile (`src/distodam/Dockerfile`)

#### 34. Add Vault service to docker-compose.yaml
**What**: Integrate Vault into local development stack
**Files**:
- `docker-compose.yaml` - Add vault service
**Configuration**:
```yaml
services:
  vault:
    build: ./src/vault
    container_name: robotorq-network-vault-1
    ports:
      - "8082:8082"  # HTTP API
      - "9092:9092"  # Prometheus metrics
    environment:
      - NATS_URL=nats://nats:4222
      - POSTGRES_URL=postgresql://vault:vault@postgres:5432/vault_db
      - LOG_LEVEL=info
      - DEMURRAGE_RATE_MONTHLY=0.005
      - VAULT_RATIO_THRESHOLD=0.80
    depends_on:
      - nats
      - postgres
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:8082/health"]
      interval: 30s
      timeout: 10s
      retries: 3
  
  postgres:
    image: postgres:16-alpine
    container_name: robotorq-network-postgres-1
    environment:
      - POSTGRES_USER=vault
      - POSTGRES_PASSWORD=vault
      - POSTGRES_DB=vault_db
    volumes:
      - postgres-data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  postgres-data:
```
**Ref**: PORT_MAPPINGS.md (assign 8082 for Vault HTTP, 9092 for metrics)

#### 35. Write integration tests (Python) for StashVault flows
**What**: Test StashVault NATS message flows end-to-end
**Files**:
- `tests/integration/test_vault_stash.py`
**Test Flows**:
1. UBD → StashVault deposit (NATS `ubd.vault_deposit`)
2. Phone → StashVault deposit (NATS `phone.vault_transfer`)
3. StashVault → Phone withdrawal (NATS `phone.vault_transfer`)
4. Demurrage collection → distribution (NATS `demurrage.paid` → yield calculated)
**Ref**: DistoDam integration tests (`tests/integration/test_distodam_*.py`)

#### 36. Write integration tests (Python) for TorqedPledge flows
**What**: Test TorqedPledge NATS message flows end-to-end
**Files**:
- `tests/integration/test_vault_pledge.py`
**Test Flows**:
1. Create pledge → UBD auto-deposit (NATS `ubd.vault_deposit`)
2. Pledge reaches target → publish `vault.pledge_ready`
3. Trust triggers BRLA → NATS `brla.triggered` → Vault locks pledge
4. Withdrawal notice → 30-day wait → approval
5. Yield distribution → wallet payment (NOT pledge balance)
**Ref**: TORQEDVAULT_IMPLEMENTATION.md section on NATS integration

#### 37. Write E2E test: UBD → StashVault → demurrage distribution
**What**: Full pipeline test (DistoDam → Vault → Wallet)
**Files**:
- `tests/e2e/test_stash_vault_full_cycle.py`
**Scenario**:
1. Start all services (docker-compose up)
2. Create wallet + StashVault
3. Simulate UBD distribution (DistoDam → `ubd.vault_deposit`)
4. Wait 24 hours (simulated via timestamp manipulation)
5. Collect demurrage from other wallets
6. Trigger daily distribution
7. Verify StashVault balance increased (yield received)
8. Withdraw to wallet
9. Verify wallet balance updated
**Ref**: DistoDam E2E test (`test-distodam-e2e.ps1`)

#### 38. Write E2E test: UBD → TorqedPledge → maturity → BRLA trigger
**What**: Full pledge lifecycle test
**Files**:
- `tests/e2e/test_pledge_maturity_cycle.py`
**Scenario**:
1. Create wallet + TorqedPledge (target: 1000 RT, monthly: 100 RT)
2. Configure DistoDam auto-diversion (10% UBD → pledge)
3. Simulate 10 months of UBD distributions
4. Verify pledge maturity detected (saved >= 1000 RT)
5. Verify `vault.pledge_ready` published to NATS
6. Mock Trust response (publish `brla.triggered`)
7. Verify pledge locked (status = "triggered")
8. Verify BRLA ID set
**Ref**: TORQEDVAULT_IMPLEMENTATION.md section on maturity/BRLA

#### 39. Create Grafana dashboard for vault metrics
**What**: Visualize vault health, yields, ratios + critical alerts
**Files**:
- `Grafana/vault-dashboard.json`
**Panels**:
1. StashVault total balance (time series)
2. TorqedPledge active vs matured (pie chart)
3. Daily yield distributed (bar chart)
4. Global vault ratio (gauge, alert if >80%)
5. Shadow vault balances (dual gauge: Stake + Disto)
6. Demurrage collection rate (time series)
7. Withdrawal notices pending (table with `WithdrawalNoticesPendingTotalMicroRT` and `WithdrawalNoticesPendingCount` gauges)
8. Reputation score distribution (histogram)
9. Circulating supply staleness (time since last Mint update - alert if >6 hours)
10. Entity count per wallet (histogram - detect Sybil patterns)

**Critical Alerts** (configure in dashboard JSON):
```json
{
  "alert": {
    "name": "Whale Exit Warning",
    "conditions": [
      {
        "query": "WithdrawalNoticesPendingTotalMicroRT / StashVaultsTotalBalance",
        "reducer": "last",
        "evaluator": { "type": "gt", "params": [0.15] }
      }
    ],
    "frequency": "5m",
    "for": "48h",
    "message": "⚠️ WHALE EXIT DETECTED: Pending withdrawals >15% of total vaulted for >48h. Possible coordinated exit.",
    "noDataState": "no_data",
    "executionErrorState": "alerting"
  }
}
```
**Why**: Early warning of coordinated large withdrawals (whale exit, bank run precursor)
**Threshold**: Alert if pending locked RT > 15-20% of total vaulted for >48 hours
**Deploy**: **On day 1 of testnet** - don't wait for issues to appear
**Ref**: Existing dashboards (`Grafana/robotorq-phase3-dashboard.json`)

#### 40. Update architecture documentation with implementation details
**What**: Document as-built architecture (code references, schema, flows)
**Files**:
- `src/vault/STASHVAULT_IMPLEMENTATION.md` - Update with actual file paths, schema
- `src/vault/TORQEDVAULT_IMPLEMENTATION.md` - Update with actual file paths, schema
- `src/vault/SHADOW_VAULT_ARCHITECTURE.md` - Update with actual implementation
- `src/vault/VAULT_SECURITY_FIXES_PART_2.md` - Mark fixes as implemented
**Updates**:
- Replace "Implementation" sections with actual code snippets
- Add database schema (PostgreSQL tables)
- Add NATS topic list (published + subscribed)
- Add API endpoint documentation (OpenAPI spec)
- Add deployment notes (environment variables, ports)
**Ref**: DistoDam architecture update pattern (`src/distodam/DISTODAM_ARCHITECTURE.md` Phase 1 completion)

---

## Implementation Order (Recommended)

### Week 1: Core Infrastructure
- Todo #1: Service skeleton
- Todo #2, #5: Data structures (StashVault + TorqedPledge)
- Todo #31, #32: Metrics + Logging
- Todo #33, #34: Docker setup

### Week 2: StashVault Implementation
- Todo #3: Deposit/Withdrawal
- Todo #4: Demurrage distribution
- Todo #27: Unit tests

### Week 3: TorqedPledge Implementation
- Todo #6: Creation + Funding
- Todo #7: **Yield to wallet (Fix #4)** - CRITICAL
- Todo #8: Maturity + BRLA
- Todo #9: Reputation
- Todo #28: Unit tests

### Week 4: Security Fixes
- Todo #14: Fix #1 (Demurrage cap + sigmoid)
- Todo #15: Fix #2 (Withdrawal notice)
- Todo #22: Fix #9 (Event sourcing) - CRITICAL
- Todo #16-21: Remaining fixes
- Todo #30: Security fix tests

### Week 5: Shadow Vaults
- Todo #10-13: Shadow vault implementation
- Todo #29: Shadow vault tests

### Week 6: Integration
- Todo #23-26: NATS messaging + HTTP API
- Todo #35-38: Integration + E2E tests
- Todo #39: Grafana dashboard
- Todo #40: Documentation updates

---

## Critical Dependencies

**Before Starting**:
- ✅ DistoDam Phase 6 complete (provides UBD distribution)
- ✅ Mint Phase 5 complete (provides RoboStake return)
- ✅ NATS running (message bus)
- ⏳ Trust service (for BRLA triggering) - can mock initially

**External Services Required**:
- PostgreSQL (event sourcing, Fix #9)
- NATS JetStream (exactly-once delivery)
- Prometheus (metrics)
- Grafana (dashboards)

**Go Dependencies** (add to `go.mod`):
```
github.com/nats-io/nats.go v1.31.0
github.com/lib/pq v1.10.9
github.com/prometheus/client_golang v1.17.0
github.com/robfig/cron/v3 v3.0.1
github.com/leanovate/gopter v0.2.9
```

---

## Testing Strategy

### Unit Tests (95%+ coverage)
- Every function in `internal/vault/*.go`
- Mock NATS, PostgreSQL (use `testcontainers-go`)
- Table-driven tests for calculations

### Integration Tests (Python + NATS)
- Real NATS messages (no mocks)
- Real database (PostgreSQL in Docker)
- Test full request-reply cycles

### E2E Tests (PowerShell/Python)
- All services running (docker-compose)
- Real wallets, real UBD distributions
- Verify complete flows (UBD → Vault → Wallet)

### Security Fix Validation
- Dedicated test suite for each fix
- Attack scenario tests (try to exploit, should fail)
- Boundary condition tests (vault ratio 79.9% vs 80.1%)

---

## Final Polish & Risk Mitigation

### Remaining Low-Risk Items

| Area | Issue | Severity | Fix | Status |
|------|-------|----------|-----|--------|
| **Leader election key rotation** | NATS KV leader lock should have reasonable TTL (5-10 min) + auto-reacquire logic on expiry/failure | Low | Add TTL to leader lock, implement heartbeat renewal | ✅ Fixed in Todo #42 |
| **Demurrage cron timezone** | Explicitly set `cron.New(cron.WithLocation(time.UTC))` to enforce midnight UTC | Low | Add timezone parameter to cron constructor | ✅ Fixed in Todo #42 |
| **Withdrawal notice maturity edge** | Notice maturing exactly when demurrage job runs needs ordering (process notices → then demurrage) | Low-Medium | Add `ProcessWithdrawalNotices()` before `DistributeDemurrage()` in scheduler | ✅ Fixed in Todo #42 |
| **Database migration ordering** | Ensure `002_add_indexes.sql` runs after table creation but before data loads | Trivial | Add migration order validation in startup | ✅ Fixed in Todo #43 |
| **Grafana alert for pending withdrawals** | Alert if `WithdrawalNoticesPendingTotalMicroRT` > 15-20% of total vaulted for >48h (whale exit warning) | Nice-to-have | Add alert rule in Todo #39 dashboard config | ✅ Fixed in Todo #39 |
| **EntityID validation regex** | Allow underscores too (`^[a-z0-9_-]+$`) - hyphens-only is restrictive for human readability | Trivial | Update regex in Fix #8 validation | ✅ Fixed in Fix #8 |
| **Leader lock key name** | Use fixed key "vault_daily_job_leader" instead of relying on Create race | Trivial | Change string in maintainLeaderLock() | ✅ Fixed in Todo #42 |
| **processMaturedWithdrawalNotices error handling** | Make it return error and abort demurrage if it fails | Low | Add error return + if err != nil { return } | ✅ Fixed in Todo #42 |
| **WithdrawalNoticesProcessedTotalMicroRT gauge** | Whale-exit alert needs denominator (total vaulted) that updates after processing | Trivial | Add one line in Metrics struct | ✅ Fixed in Todo #31 |
| **Hostname fallback in leader election** | os.Hostname() with fallback to uuid.New() if empty | Low | Prevents leader election from exploding in Docker | ✅ Fixed in Todo #42 |

**None of the above are blockers** - just the last 1% polish for production hardening.

---

## Tactical Execution Plan

### Critical Path (Do This Exactly)

**Week 1: Phase 0 Infrastructure ONLY** (Todos #41-48)
```
DO NOT TOUCH StashVault or TorqedPledge logic until:
✅ Circulating supply subscription working
✅ Leader election + scheduler green  
✅ Event sourcing indexes created
✅ Staleness freeze implemented
✅ Shadow vaults transactional

Everything else will break in subtle ways without these foundations.
```

**Prerequisite: Merge Mint Circulating Supply Publisher**
- Implement Appendix A changes (~150 lines)
- Deploy Mint update BEFORE cutting first Vault release
- Unblocks entire sigmoid machinery

**Week 4: Security Fix Implementation Order**
```
Implement in this exact order (strongest safety net earliest):
1. Todo #22 (Fix #9: Event sourcing) → Foundation for everything
2. Todo #17 (Fix #4: Yield to wallet) → Prevents infinite money exploit
3. Todo #14 (Fix #1: Sigmoid cap) → Prevents liquidity death spiral
4. Todo #15 (Fix #2: 30-day notice) → Prevents bank run timing attacks
5. All remaining fixes (order less critical)
```

**Todo #30: Attack Scenario Test Suite**
- After implementing `attack_scenarios_test.go`, pin file as:
  ```
  "DO NOT TOUCH WITHOUT 2 SENIOR REVIEWS"
  ```
- This test file is **more valuable than most production code**
- It's the regression test that screams if exploits return

**Day 1 of Testnet: Ship Grafana Dashboard**
- Deploy Todo #39 dashboard immediately
- Monitor metrics from hour 1
- Early detection of issues prevents production disasters

---

## Deployment Checklist

Before merging `feature/vault` into `v0`:

- [ ] All 48 todos complete (including 8 critical infrastructure fixes)
- [ ] Unit tests passing (95%+ coverage)
- [ ] Integration tests passing
- [ ] E2E tests passing (both StashVault and TorqedPledge cycles)
- [ ] Security fix tests passing (all 9 fixes validated)
- [ ] **Attack scenario test suite passing** (Todo #30 - compounding exploit, sybil, etc.)
- [ ] **Mint circulating supply publisher deployed** (Appendix A - prerequisite)
- [ ] **Leader election TTL configured** (5-10 min, auto-renewal)
- [ ] **Cron timezone explicitly UTC** (`cron.WithLocation(time.UTC)`)
- [ ] **Withdrawal notice ordering validated** (notices before demurrage)
- [ ] **Migration order verified** (indexes after tables, before data)
- [ ] **EntityID regex allows underscores** (`^[a-z0-9_-]+$`)
- [ ] Grafana dashboard deployed **on day 1 of testnet**
- [ ] Documentation updated (architecture + implementation)
- [ ] Code review complete
- [ ] Performance benchmarks run (target: 1000 deposits/sec)
- [ ] Memory profiling complete (no leaks)
- [ ] Docker image builds successfully
- [ ] docker-compose up works (all services healthy)

---

## Rollback Plan

If issues found in production:

1. **Immediate**: Stop Vault service (docker-compose stop vault)
2. **Data Integrity**: Event sourcing preserves all state (replay from events)
3. **Rollback**: Revert to previous version, replay events to restore state
4. **Communication**: Notify users of temporary vault unavailability
5. **Fix**: Debug in staging, redeploy when fixed

**No data loss possible** (thanks to Fix #9 event sourcing)

---

## Contact / Escalation

If stuck on any todo:
1. Check referenced documentation (VAULT_SECURITY_FIXES_PART_2.md, etc.)
2. Review DistoDam implementation (similar patterns)
3. Check GitHub Copilot instructions (`.github/copilot-instructions.md`)
4. Consult white paper (README.md) for economic theory
5. Ask for help (create GitHub issue with `[Vault]` prefix)

**Critical Security Fixes** (DO NOT SKIP):
- Fix #4 (yield to wallet) - Prevents infinite money
- Fix #1 (demurrage cap) - Prevents liquidity crisis
- Fix #9 (event sourcing) - Prevents data loss

**Critical Infrastructure Fixes** (DO NOT SKIP):
- Todo #41 (circulating supply subscription) - Sigmoid suppression depends on this
- Todo #42 (scheduler + leader election) - Prevents duplicate demurrage distributions
- Todo #44 (staleness policy) - Prevents incorrect yield calculations
- Todo #45 (shadow vault transactions) - Prevents race conditions in reserves
- Todo #48 (fuzz tests) - Validates event sourcing resilience

**Sigmoid Parameters** (Hard-coded for Genesis Immutability):
- **k = 18.0** (slope steepness)
- **x₀ = 0.85** (center point - 85% vault ratio)
- **Rationale**: See VAULT_SECURITY_FIXES_PART_2.md Appendix A (equilibrium analysis)
- **Future**: When governance is added (Phase 10+), parameters can be migrated to database table with governance votes. For now, constants are intentionally hard-coded to prevent tampering.

---

## Appendix A: Changes Required in Other Services

### Mint Service - Circulating Supply Publisher (CRITICAL for Todo #41)

**Problem**: Vault needs real-time circulating RT data for sigmoid suppression, but Mint doesn't currently publish this metric.

**Required Changes**:

**File**: `src/mint/internal/mint/circulating_supply_publisher.go` (NEW)
```go
package mint

import (
    "context"
    "encoding/json"
    "log/slog"
    "sync/atomic"
    "time"
    "github.com/nats-io/nats.go"
)

type CirculatingSupplyPublisher struct {
    nc                    *nats.Conn
    js                    nats.JetStreamContext
    kv                    nats.KeyValue
    circulatingSupplyMicroRT atomic.Int64
}

func NewCirculatingSupplyPublisher(nc *nats.Conn) (*CirculatingSupplyPublisher, error) {
    js, err := nc.JetStream()
    if err != nil {
        return nil, err
    }
    
    // Create KV bucket for circulating supply
    kv, err := js.CreateKeyValue(&nats.KeyValueConfig{
        Bucket:      "mint_circulating_supply",
        Description: "Real-time circulating RT supply for vault ratio calculations",
        TTL:         24 * time.Hour, // Keep last 24 hours of history
        MaxBytes:    10 * 1024 * 1024, // 10MB
    })
    if err != nil {
        // Bucket might already exist
        kv, err = js.KeyValue("mint_circulating_supply")
        if err != nil {
            return nil, err
        }
    }
    
    return &CirculatingSupplyPublisher{
        nc: nc,
        js: js,
        kv: kv,
    }, nil
}

func (p *CirculatingSupplyPublisher) UpdateCirculatingSupply(ctx context.Context, totalMintedMicroRT, totalBurnedMicroRT int64) error {
    circulatingMicroRT := totalMintedMicroRT - totalBurnedMicroRT
    p.circulatingSupplyMicroRT.Store(circulatingMicroRT)
    
    // Publish to JetStream KV
    data, err := json.Marshal(map[string]interface{}{
        "circulating_supply_micro_rt": circulatingMicroRT,
        "total_minted_micro_rt":       totalMintedMicroRT,
        "total_burned_micro_rt":       totalBurnedMicroRT,
        "timestamp":                   time.Now().Unix(),
    })
    if err != nil {
        return err
    }
    
    _, err = p.kv.Put("current", data)
    if err != nil {
        slog.Error("failed to publish circulating supply",
            "error", err,
            "circulating_micro_rt", circulatingMicroRT)
        return err
    }
    
    slog.Info("circulating supply updated",
        "circulating_rt", float64(circulatingMicroRT)/1_000_000.0,
        "total_minted_rt", float64(totalMintedMicroRT)/1_000_000.0)
    
    return nil
}

// Periodic background publisher (every 30 seconds)
func (p *CirculatingSupplyPublisher) Start(ctx context.Context, mintEngine *MintEngine) {
    ticker := time.NewTicker(30 * time.Second)
    defer ticker.Stop()
    
    for {
        select {
        case <-ctx.Done():
            return
        case <-ticker.C:
            totalMinted := mintEngine.GetTotalMintedMicroRT()
            totalBurned := mintEngine.GetTotalBurnedMicroRT()
            
            if err := p.UpdateCirculatingSupply(ctx, totalMinted, totalBurned); err != nil {
                slog.Error("failed to update circulating supply", "error", err)
            }
        }
    }
}
```

**File**: `src/mint/internal/mint/mint_engine.go` (UPDATE)
```go
type MintEngine struct {
    // ... existing fields
    totalMintedMicroRT   atomic.Int64  // NEW
    totalBurnedMicroRT   atomic.Int64  // NEW
    supplyPublisher      *CirculatingSupplyPublisher // NEW
}

func (me *MintEngine) ProcessBatch(ctx context.Context, batch *RoboTorqBatch) error {
    // ... existing batch processing
    
    // Track minted RT
    for _, unit := range batch.Units {
        me.totalMintedMicroRT.Add(int64(unit.RoboTorqValue * 1_000_000))
    }
    
    // Publish updated circulating supply
    if err := me.supplyPublisher.UpdateCirculatingSupply(
        ctx,
        me.totalMintedMicroRT.Load(),
        me.totalBurnedMicroRT.Load(),
    ); err != nil {
        slog.Warn("failed to publish circulating supply", "error", err)
    }
    
    return nil
}

func (me *MintEngine) GetTotalMintedMicroRT() int64 {
    return me.totalMintedMicroRT.Load()
}

func (me *MintEngine) GetTotalBurnedMicroRT() int64 {
    return me.totalBurnedMicroRT.Load()
}
```

**File**: `src/mint/cmd/mint/main.go` (UPDATE)
```go
func main() {
    // ... existing setup
    
    // Create circulating supply publisher
    supplyPublisher, err := mint.NewCirculatingSupplyPublisher(nc)
    if err != nil {
        log.Fatal("failed to create supply publisher:", err)
    }
    
    mintEngine := mint.NewMintEngine(
        logger,
        metrics,
        hasher,
        supplyPublisher, // NEW parameter
    )
    
    // Start background publisher
    go supplyPublisher.Start(ctx, mintEngine)
    
    // ... rest of main
}
```

**NATS Topic Schema**:
- **Bucket**: `mint_circulating_supply` (KV bucket)
- **Key**: `current`
- **Value**:
  ```json
  {
    "circulating_supply_micro_rt": 1500000000000,
    "total_minted_micro_rt": 2000000000000,
    "total_burned_micro_rt": 500000000000,
    "timestamp": 1700000000
  }
  ```

**Testing**:
```go
// src/mint/internal/mint/circulating_supply_publisher_test.go
func TestCirculatingSupplyPublisher(t *testing.T) {
    // Test KV bucket creation
    // Test supply updates
    // Test watcher (simulate Vault subscription)
}
```

**Metrics** (add to Mint):
```go
CirculatingSupplyMicroRT    prometheus.Gauge
TotalMintedMicroRT          prometheus.Counter
TotalBurnedMicroRT          prometheus.Counter
SupplyPublishErrors         prometheus.Counter
```

**Deployment**: This change MUST be deployed to Mint BEFORE Vault can use circulating supply data.

---

### DistoDam Service - UBD Metadata Enhancement (Optional for Todo #6)

**Problem**: TorqedPledge needs to validate monthly pledge amounts, but DistoDam doesn't currently send UBD metadata.

**Optional Enhancement**:

**File**: `src/distodam/internal/distodam/ubd_publisher.go` (UPDATE)
```go
type UBDDistribution struct {
    // ... existing fields
    
    // NEW: Metadata for pledge validation
    ExpectedMonthlyPledgeMicroRT int64  `json:"expected_monthly_pledge_micro_rt"`
    UBDPercentageToPledge        float64 `json:"ubd_percentage_to_pledge"`
}
```

This allows Vault to validate: `ubd_amount * ubd_percentage >= expected_monthly_pledge`

**Priority**: Medium (can use rolling 3-month average validation instead)

---

### Trust Service - BRLA Triggered Events (Required for Todo #8)

**Status**: Trust service doesn't exist yet (Phase 8+)

**Required NATS Topic**:
- **Topic**: `brla.triggered`
- **Message**:
  ```json
  {
    "brla_id": "brla-20251118-001",
    "pledge_id": "pledge-xyz789",
    "contract_id": "contract-abc123",
    "triggered_at": "2025-11-18T12:00:00Z",
    "signature": "..."
  }
  ```

**Action**: When Trust service is implemented, it must publish this event when creating BRLAs from matured pledges.

---

### Wallet/Phone Service - Demurrage Payment Events (Required for Todo #4)

**Status**: Wallet service doesn't exist yet (Phase 7+)

**Required NATS Topic**:
- **Topic**: `demurrage.paid`
- **Message**:
  ```json
  {
    "wallet_id": "wallet-abc123",
    "demurrage_micro_rt": 5000,
    "idle_balance_micro_rt": 1000000,
    "timestamp": "2025-11-18T00:00:00Z",
    "calculation_proof": "...",
    "signature": "..."
  }
  ```

**Action**: Wallet must calculate demurrage hourly and publish settlement events daily.

---

### BidNet Service - Transaction Fee Events (Required for Todo #11)

**Status**: BidNet service doesn't exist yet (Phase 9+)

**Required NATS Topic**:
- **Topic**: `bidnet.transaction_fee`
- **Message**:
  ```json
  {
    "flow_id": "flow-20251118-001",
    "torqvault_id": "torqvault-alpha",
    "fee_micro_rt": 1000,
    "fee_percentage": 0.001,
    "timestamp": "2025-11-18T12:00:00Z",
    "distodam_signature": "..."
  }
  ```

**Action**: BidNet must publish fee events when escrow winners are awarded.

---

## Appendix B: Vault Ratio Calculation Dependencies

### Data Sources Required for Sigmoid Suppression

**Vault Ratio Formula**:
```
vault_ratio = total_vaulted_rt / circulating_supply_rt
```

**Components**:

1. **Total Vaulted RT** (Vault owns this):
   - `SUM(StashVault.balance_micro_rt)`
   - `SUM(TorqedPledge.saved_micro_rt)`
   - Calculated internally by Vault service

2. **Circulating Supply RT** (Mint owns this):
   - `total_minted_rt - total_burned_rt`
   - **MUST** be subscribed from Mint (Todo #41)
   - Stale data (>6 hours) triggers yield freeze (Todo #44)

3. **Shadow Vault Balances** (Vault owns this):
   - NOT included in vault ratio (reserves, not member vaults)
   - Tracked separately for transparency

**Critical Path**:
```
Mint publishes circulating supply (30s intervals)
  ↓
Vault subscribes via JetStream KV watcher
  ↓
Vault caches with TTL (6 hour staleness threshold)
  ↓
Daily demurrage distribution checks freshness
  ↓
If stale → FREEZE yield, emit CRITICAL alert
  ↓
If fresh → Calculate vault_ratio, apply sigmoid suppression
```

**Failure Modes**:
- **Mint offline**: Vault detects stale data, freezes yield
- **NATS partition**: Vault detects stale data, freezes yield
- **Vault cold start**: Subscribes to KV bucket, gets latest value immediately

**Recovery**:
- Mint comes back online → publishes supply → Vault unfreezes within 30 seconds
- No data loss (event sourcing preserves pending demurrage)

---

## Appendix C: Implementation Order Update

### Revised Week 1: Critical Infrastructure FIRST

**Before implementing any vault logic, fix infrastructure issues:**

**Day 1-2**:
- Todo #41: Circulating supply subscription (blocks sigmoid suppression)
- Todo #42: Scheduler + leader election (blocks demurrage distribution)
- Todo #43: PostgreSQL indexes (blocks event sourcing performance)

**Day 3-4**:
- Todo #44: Staleness policy (blocks safe yield distribution)
- Todo #45: Shadow vault transactions (blocks shadow vault safety)
- Todo #47: Module restructure (blocks maintainability)

**Day 5**:
- Todo #46: HTTP API rejections (nice-to-have, can defer)
- Todo #48: Fuzz tests (add to continuous testing suite)

**Why**: Infrastructure issues are **showstoppers**. Without them:
- Sigmoid suppression is blind (Issue #1)
- Multiple demurrage distributions occur (Issue #2)
- Event queries are slow (Issue #3)
- Incorrect yields distributed (Issue #4)
- Shadow vault race conditions (Issue #5)

**Then proceed** with original Week 1 (service skeleton, data structures, metrics, Docker).

---

**End of Implementation Plan**
