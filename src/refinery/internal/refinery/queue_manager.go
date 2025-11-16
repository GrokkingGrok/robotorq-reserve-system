// internal/refinery/queue_manager.go
// Manages hash queue with thread-safe operations
// Phase 2: Hash-only storage (NO full JTU data)

package refinery

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// QueueManager implements thread-safe buffering for both Phase 1 (JouleTorqUnits)
// and Phase 2 (hash-only) architectures.
//
// Phase 2 Hash-Based Queue: ACTIVE
// - AddHash(hash, contractID, diggerID): Add hash to FIFO queue
// - GetHashes(count): Blocking retrieval of N hashes (waits until available)
// - Metrics: queue_hashes_queued_total, queue_depth_hashes
//
// Phase 1 Unit-Based Queue: DEPRECATED
// - AddUnit(unit): Returns ErrDeprecated
// - GetUnit(): Returns ErrDeprecated
// - Kept for compatibility during Phase 2 migration
var (
	// hashQueueGauge tracks current hash queue size
	hashQueueGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_hash_queue_size",
		Help: "Current number of hashes in the queue",
	})

	// hashesQueuedTotal tracks total hashes added to queue
	hashesQueuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_hashes_queued_total",
		Help: "Total hashes queued for ingot assembly",
	})

	// hashesDequeuedTotal tracks total hashes consumed
	hashesDequeuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_hashes_dequeued_total",
		Help: "Total hashes dequeued for merkle tree building",
	})
)

func init() {
	// Register Prometheus metrics
	prometheus.MustRegister(hashQueueGauge)
	prometheus.MustRegister(hashesQueuedTotal)
	prometheus.MustRegister(hashesDequeuedTotal)
}

// HashEntry represents a single hash in the queue
// Phase 2: Store hashes only, NOT full JTU data (98% bandwidth reduction)
type HashEntry struct {
	Hash       string    `json:"hash"`        // 32-byte hex SHA256 hash of JTU
	ContractID string    `json:"contract_id"` // Which contract generated this hash
	DiggerID   string    `json:"digger_id"`   // Which Digger sent this hash
	Timestamp  time.Time `json:"timestamp"`   // When hash was received
	Index      int64     `json:"index"`       // Sequential index in queue (for FIFO ordering)
}

// QueueManager handles buffered queue for hash entries
// Phase 2: Changed from JouleTorqUnit queue to hash-only queue
type QueueManager struct {
	// Phase 2: Hash-based queue (ACTIVE)
	hashQueue []HashEntry     // FIFO hash storage
	hashIndex int64           // Sequential indexing for FIFO order
	capacity  int             // Max queue size
	mu        sync.Mutex      // Protects hashQueue
	notEmpty  *sync.Cond      // Signals when hashes available
	ctx       context.Context // Graceful shutdown support
	nextIndex int64           // Auto-incrementing index for FIFO
}

// NewQueueManager creates a new hash-based queue manager
func NewQueueManager(ctx context.Context, capacity int) *QueueManager {
	slog.Info("initializing queue manager",
		"hash_capacity", capacity,
	)

	qm := &QueueManager{
		hashQueue: make([]HashEntry, 0, capacity),
		capacity:  capacity,
		ctx:       ctx,
		nextIndex: 0,
	}
	qm.notEmpty = sync.NewCond(&qm.mu)

	return qm
}

// AddHash adds a hash to the queue (Phase 2 hash-only architecture).
// Non-blocking with backpressure: returns ErrQueueFull when at capacity.
//
// Thread-safe: uses mutex for concurrent access.
// AddHash adds a hash to the queue (non-blocking with backpressure)
func (qm *QueueManager) AddHash(hash, contractID, diggerID string) error {
	qm.mu.Lock()
	defer qm.mu.Unlock()

	// Check if shutting down
	select {
	case <-qm.ctx.Done():
		return models.ErrQueueShuttingDown
	default:
	}

	// Check capacity
	if len(qm.hashQueue) >= qm.capacity {
		slog.Warn("hash queue full, rejecting item",
			"contract", contractID,
			"queue_size", len(qm.hashQueue),
		)
		return models.ErrQueueFull
	}

	// Add hash entry
	entry := HashEntry{
		Hash:       hash,
		ContractID: contractID,
		DiggerID:   diggerID,
		Timestamp:  time.Now(),
		Index:      qm.nextIndex,
	}
	qm.nextIndex++

	qm.hashQueue = append(qm.hashQueue, entry)

	// Update metrics
	hashesQueuedTotal.Inc()
	hashQueueGauge.Set(float64(len(qm.hashQueue)))

	// Signal waiting consumers
	qm.notEmpty.Signal()

	slog.Debug("hash added to queue",
		"contract", contractID,
		"digger", diggerID,
		"queue_size", len(qm.hashQueue),
	)

	return nil
}

// GetHashes retrieves N hashes from the queue (Phase 2 hash-only architecture).
// Blocks using sync.Cond until N hashes are available OR context is cancelled.
//
// Returns:
// - []HashEntry: Exactly N hashes in FIFO order
// - error: ctx.Err() if cancelled, nil on success
//
// Thread-safe: uses mutex and condition variable.
// GetHashes retrieves N hashes from queue (BLOCKS until N available)
func (qm *QueueManager) GetHashes(count int) ([]HashEntry, error) {
	qm.mu.Lock()
	defer qm.mu.Unlock()

	// Wait until we have enough hashes OR context cancelled
	for len(qm.hashQueue) < count {
		select {
		case <-qm.ctx.Done():
			// Shutting down - return what we have
			if len(qm.hashQueue) > 0 {
				slog.Warn("queue shutting down, returning partial batch",
					"requested", count,
					"available", len(qm.hashQueue),
				)
				hashes := make([]HashEntry, len(qm.hashQueue))
				copy(hashes, qm.hashQueue)
				qm.hashQueue = qm.hashQueue[:0]
				return hashes, models.ErrQueueShuttingDown
			}
			return nil, models.ErrQueueEmpty

		default:
			// Wait for more hashes
			qm.notEmpty.Wait()
		}
	}

	// Extract first N hashes (FIFO)
	hashes := make([]HashEntry, count)
	copy(hashes, qm.hashQueue[:count])
	qm.hashQueue = qm.hashQueue[count:]

	// Update metrics
	hashesDequeuedTotal.Add(float64(count))
	hashQueueGauge.Set(float64(len(qm.hashQueue)))

	slog.Info("hashes dequeued from queue",
		"count", count,
		"remaining", len(qm.hashQueue),
	)

	return hashes, nil
}

// GetQueueSize returns current queue size (thread-safe)
func (qm *QueueManager) GetQueueSize() int {
	qm.mu.Lock()
	defer qm.mu.Unlock()
	return len(qm.hashQueue)
}

// GetCapacity returns the total capacity of the queue
func (qm *QueueManager) GetCapacity() int {
	return qm.capacity
}

// Close gracefully shuts down the queue manager
func (qm *QueueManager) Close() {
	slog.Info("closing queue manager",
		"hashes_remaining", len(qm.hashQueue),
	)

	qm.mu.Lock()
	defer qm.mu.Unlock()

	// Broadcast to all waiting consumers
	qm.notEmpty.Broadcast()
}

// DEPRECATED: Old unit-based methods (will be removed after Phase 2 complete)
// AddUnit - NO LONGER USED in Phase 2
func (qm *QueueManager) AddUnit(unit *models.JouleTorqUnit) error {
	slog.Error("AddUnit called but Phase 2 uses hash-only queue",
		"contract", unit.ContractID,
	)
	return models.ErrDeprecated
}

// GetUnit - NO LONGER USED in Phase 2
func (qm *QueueManager) GetUnit() (*models.JouleTorqUnit, error) {
	slog.Error("GetUnit called but Phase 2 uses hash-only queue")
	return nil, models.ErrDeprecated
}
