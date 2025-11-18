// Package mint provides the core interfaces and contracts for the Mint service.
// These interfaces define the API boundaries between components, enabling
// modular design, testability, and future upgrades (e.g., SimpleBatchHasher → FullMerkleBuilder).
package mint

import (
	"context"
	"errors"
	"time"

	"github.com/nats-io/nats.go"
)

// ─────────────────────────────────────────────────────────────
// Common Errors
// ─────────────────────────────────────────────────────────────

var (
	// ErrBufferFull indicates the ingot buffer is at capacity.
	ErrBufferFull = errors.New("ingot buffer is full")

	// ErrBufferEmpty indicates no ingots available in buffer.
	ErrBufferEmpty = errors.New("ingot buffer is empty")

	// ErrInvalidIngot indicates ingot validation failed.
	ErrInvalidIngot = errors.New("invalid ingot")

	// ErrNatsDisconnected indicates NATS connection is lost.
	ErrNatsDisconnected = errors.New("nats connection lost")
)

// ─────────────────────────────────────────────────────────────
// Core Component Interfaces
// ─────────────────────────────────────────────────────────────

// IngotReceiver handles HTTP ingot reception from Refinery.
// Responsibilities:
//   - Validate incoming TokenTorqIngot payloads
//   - Push valid ingots to IngotBuffer
//   - Return HTTP 429 on backpressure
//   - Update Prometheus metrics
type IngotReceiver interface {
	// ReceiveIngot validates and queues a single TokenTorqIngot.
	// Returns error if validation fails or buffer is full.
	ReceiveIngot(ingot *TokenTorqIngot) error

	// Start begins the HTTP server listening on configured port.
	// Blocks until context is cancelled or server fails.
	Start(ctx context.Context) error

	// Shutdown gracefully stops the HTTP server within timeout.
	Shutdown(ctx context.Context) error
}

// IngotBuffer provides thread-safe buffering for incoming ingots.
// Responsibilities:
//   - Thread-safe Push/Pop operations
//   - Capacity management (default 100k)
//   - Backpressure signaling
//   - Atomic counter tracking for metrics
type IngotBuffer interface {
	// Push adds an ingot to the buffer.
	// Returns error if buffer is at capacity.
	Push(ingot *TokenTorqIngot) error

	// Pop retrieves the next ingot from buffer.
	// Blocks until ingot available or context cancelled.
	Pop(ctx context.Context) (*TokenTorqIngot, error)

	// Len returns current buffer depth (thread-safe).
	Len() int

	// Cap returns buffer capacity.
	Cap() int

	// Drain empties the buffer and returns all remaining ingots.
	// Used during graceful shutdown.
	Drain() []*TokenTorqIngot
}

// BatchAggregator accumulates ingots into batches.
// Responsibilities:
//   - Accumulate to 1000-ingot threshold
//   - Trigger MintEngine on full batch
//   - Interval-based partial flush (configurable, default 5s)
//   - Track batch timing for latency metrics
type BatchAggregator interface {
	// Start begins the aggregation loop.
	// Continuously pulls from buffer, builds batches, triggers MintEngine.
	// Blocks until context is cancelled.
	Start(ctx context.Context) error

	// Flush immediately processes current batch (even if partial).
	// Used during shutdown to prevent data loss.
	Flush() error

	// GetAccumulatedCount returns current batch size (for metrics/testing).
	GetAccumulatedCount() int
}

// BatchHasher defines the pluggable interface for batch processing.
// Current implementation: SimpleBatchHasher (SHA256 aggregation)
// Future implementation: FullMerkleBuilder (complete Merkle tree with proofs)
//
// Design principle: Swap implementations without changing MintEngine code.
type BatchHasher interface {
	// Hash processes a batch of ingots and returns batch hash + individual stakes.
	//
	// PHASE 6 UPDATE: Returns IngotStakes[] instead of summing RoboStake.
	// This preserves proof chain granularity for DistoDam dual-vault architecture.
	//
	// Returns:
	//   - batchHash: cryptographic hash of batch (SHA256 now, Merkle root later)
	//   - ingotStakes: array of individual ingot stakes (preserves contract provenance)
	//   - error: if hashing fails
	Hash(batch []*TokenTorqIngot) (batchHash string, ingotStakes []IngotStake, err error)
}

// MintEngine coordinates batch processing and event generation.
// Responsibilities:
//   - Use BatchHasher to process batches
//   - Generate MintEvent structures
//   - Publish events via DistoDamClient
//   - Track metrics (batches processed, RoboTorq aggregated)
type MintEngine interface {
	// ProcessBatch takes a batch of ingots, hashes them, creates MintEvent,
	// and publishes to DistoDam via NATS.
	ProcessBatch(ctx context.Context, batch []*TokenTorqIngot) error

	// GetTotalProcessed returns cumulative ingots processed (for metrics).
	GetTotalProcessed() int64

	// GetTotalRoboAggregated returns cumulative RoboTorq aggregated (for metrics).
	GetTotalRoboAggregated() float64
}

// DistoDamClient handles NATS publishing to DistoDam instances.
// Responsibilities:
//   - Publish MintEvent to 'mint.batches' topic (broadcast)
//   - Retry with exponential backoff (3 retries, 100ms base)
//   - Connection management and health monitoring
//   - Graceful degradation on publish failures
type DistoDamClient interface {
	// Publish sends a MintEvent to the mint.batches topic.
	// All subscribed DistoDams will receive the event.
	// Retries on transient failures (connection loss, timeout).
	Publish(ctx context.Context, event *MintEvent) error

	// Connect establishes NATS connection.
	// Called during initialization.
	Connect() error

	// Close gracefully shuts down NATS connection.
	// Called during shutdown.
	Close() error

	// IsConnected returns current connection status.
	IsConnected() bool

	// GetConnection returns the underlying NATS connection.
	// Used by IngotReceiver to subscribe to mint.ingots topic.
	GetConnection() *nats.Conn
}

// ─────────────────────────────────────────────────────────────
// Data Models
// ─────────────────────────────────────────────────────────────

// TokenTorqIngot represents a single ingot received from Refinery.
// This is the INPUT to Mint - already assembled by Refinery from JouleTorqOre.
//
// **REFACTORED VERSION** (currency-refactor branch):
// An ingot now contains 3,600 JouleTorqUnits (atomic token proofs), not just aggregated joules.
// This enables full merkle tree verification from individual tokens → ingots → RoboTorq.
//
// Structure (merkle tree branch):
//   - Units: 3,600 JouleTorqUnits (leaf nodes in merkle tree)
//   - BranchHash: SHA256 of all unit hashes (merkle branch)
//   - Energy proof (JouleTorq - sum of all Units[].JoulesConsumed ≈ 3600J)
//   - Robot stake paid (sum of all Units[].RoboStakePaid)
//   - Metadata for traceability (IngotID, ContractIDs, MintedAt)
type TokenTorqIngot struct {
	// IngotID is a unique identifier for this ingot (UUID)
	IngotID string `json:"ingot_id"`

	// Units contains exactly 3,600 JouleTorqUnits (atomic token proofs)
	// Each unit represents ~1 joule of robotic work
	// NOTE: This is a pointer to allow JSON unmarshaling from NATS messages
	Units []*JouleTorqUnit `json:"units"`

	// JouleTorqTotal is the total joules in this ingot (~3600J)
	// Calculated as sum(Units[].JoulesConsumed)
	JouleTorqTotal float64 `json:"joule_torq_total"`

	// RoboStakeTotal is the accumulated RoboTorq from all units
	// Calculated as sum(Units[].RoboStakePaid)
	RoboStakeTotal float64 `json:"robo_stake_total"`

	// ContractIDs lists all unique contracts that contributed to this ingot
	// Extracted from Units[].ContractID (deduplicated)
	ContractIDs []string `json:"contract_ids"`

	// BranchHash is the SHA256 merkle tree branch hash
	// Calculated as hash(Units[0].Hash + Units[1].Hash + ... + Units[3599].Hash)
	// This becomes a leaf in the RoboTorqUnit merkle tree
	BranchHash string `json:"branch_hash"`

	// MintedAt is when this ingot was assembled by Refinery
	MintedAt time.Time `json:"minted_at"`
}

// JouleTorqUnit represents a single atomic proof of robotic work (merkle tree leaf)
// This is copied from refinery/internal/models for NATS deserialization
//
// TODO(currency-refactor): Consider shared models package to avoid duplication
type JouleTorqUnit struct {
	TokenID        string    `json:"token_id"`        // Unique identifier
	ContractID     string    `json:"contract_id"`     // BRLA reference
	MilestoneIndex int       `json:"milestone_index"` // Progress tracker
	TokenIndex     int       `json:"token_index"`     // Position in milestone
	JoulesConsumed float64   `json:"joules_consumed"` // Energy expended (~1J)
	RoboStakePaid  float64   `json:"robo_stake_paid"` // RT value
	DiggerID       string    `json:"digger_id"`       // Executor ID
	Timestamp      time.Time `json:"timestamp"`       // Unix timestamp
	Signature      string    `json:"signature"`       // Dilithium5 proof
	DiggerPubKey   string    `json:"digger_pub_key"`  // Verification key
	Hash           string    `json:"hash"`            // SHA256 merkle leaf hash
}

// MintEvent is the OUTPUT from Mint - published to DistoDam via NATS.
//
// Economic model: NO NEW ROBOTORQ IS MINTED.
// Mint aggregates RoboStake from ingots and broadcasts individual stakes
// to DistoDam for dual-vault deposit (StakeVault/DistoVault).
//
// CRITICAL CHANGE (Phase 6 - Dual-Vault Architecture):
// Instead of summing RoboStake (loses granularity), we now send individual
// IngotStakes[] to preserve proof chain and enable loan repayment tracking.
//
// Structure:
//   - BatchHash: cryptographic proof of batch (simple SHA256 now, Merkle root later)
//   - IngotStakes: array of individual ingot stakes (preserves granularity)
//   - IngotsProcessed: count of ingots in batch
//   - Metadata: BatchID, Timestamp for tracking
type MintEvent struct {
	// Cryptographic proof
	BatchHash string `json:"batch_hash"` // SHA256 of batch data (future: Merkle root)

	// Individual ingot stakes (CRITICAL: Each ingot processed separately, not summed)
	IngotStakes []IngotStake `json:"ingot_stakes"` // Array of stakes, preserves proof chain

	// Metadata
	IngotsProcessed int       `json:"ingots_count"` // Batch size (usually 1000)
	BatchID         string    `json:"batch_id"`     // Unique identifier
	Timestamp       time.Time `json:"timestamp"`    // UTC timestamp

	// DEPRECATED (kept for backward compatibility - will be removed in Phase 7)
	TotalRoboTorq float64 `json:"total_robo,omitempty"` // Deprecated: Use sum(IngotStakes[].RoboStakeTotal)
	SaleValueUSD  float64 `json:"sale_value,omitempty"` // Deprecated: Accounting moved to separate service
}

// IngotStake represents the RoboStake from a single ingot
// CRITICAL: Preserves proof chain (contract → ore → ingot → stake)
type IngotStake struct {
	IngotID        string   `json:"ingot_id"`         // Unique ingot identifier
	RoboStakeTotal float64  `json:"robo_stake_total"` // RoboStake for THIS ingot (circulating RT)
	ContractIDs    []string `json:"contract_ids"`     // Contracts that contributed to this ingot
}

// ─────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────

// Config holds all Mint service configuration.
// Loaded from environment variables with sensible defaults.
type Config struct {
	// HTTP server
	HTTPPort string // Default: "8080"

	// NATS connection
	NatsURL string // Default: "nats://nats:4222"

	// Batch processing
	BatchSize     int           // Default: 1000 ingots
	FlushInterval time.Duration // Default: 5s

	// Buffering
	BufferCapacity int // Default: 100000 ingots

	// Retry configuration
	NatsRetries     int           // Default: 3
	NatsBackoffBase time.Duration // Default: 100ms
}

// ─────────────────────────────────────────────────────────────
// Notes on Future Upgrades
// ─────────────────────────────────────────────────────────────
//
// Upgrading SimpleBatchHasher → FullMerkleBuilder:
//
// 1. Create new implementation of BatchHasher interface
// 2. Implement full Merkle tree construction in Hash() method
// 3. Return Merkle root as batchHash
// 4. Optionally extend MintEvent to include Merkle proofs
// 5. Swap implementation in main.go (no other code changes needed)
//
// Example:
//   // Old (v0)
//   hasher := NewSimpleBatchHasher()
//
//   // New (v1 with Merkle)
//   hasher := NewFullMerkleBuilder(crypto.SHA256)
//
//   // MintEngine doesn't change!
//   engine := NewMintEngine(hasher, client)
//
// This pluggable design ensures we can add cryptographic rigor
// without refactoring the entire service.
