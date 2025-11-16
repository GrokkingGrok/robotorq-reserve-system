// Package mint implements the core Mint service components.
package mint

import (
	"context"
	"fmt"
	"log/slog"
	"sync"

	"b2b/mint/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// IngotHashQueue stores IngotHashEntry items in a thread-safe FIFO queue.
// It accumulates exactly 1000 ingot hashes before triggering Level 2 merkle tree building.
//
// Key differences from Refinery's QueueManager:
//   - Stores hash entries (not full ingots) for memory efficiency
//   - Batch size: 1000 ingot hashes (vs 3600 JTU hashes)
//   - Blocking GetIngotHashes() waits for full batch
//   - Context cancellation support for graceful shutdown
type IngotHashQueue struct {
	entries   []*models.IngotHashEntry
	mu        sync.Mutex
	notFull   *sync.Cond // Signals when batch is ready (1000 entries)
	capacity  int
	batchSize int // Number of entries to accumulate before returning (1000)
	logger    *slog.Logger
	metrics   *IngotHashQueueMetrics
}

// IngotHashQueueMetrics tracks queue operations
type IngotHashQueueMetrics struct {
	QueueDepth         prometheus.Gauge
	EntriesAddedTotal  prometheus.Counter
	BatchesReadyTotal  prometheus.Counter
	ContextCancelled   prometheus.Counter
	AddBlockedTotal    prometheus.Counter // When queue is full
	GetBlockedDuration prometheus.Histogram
}

// NewIngotHashQueue creates a new IngotHashQueue
//
// Parameters:
//   - capacity: Maximum queue size (default: 2000, allowing 2 batches in flight)
//   - batchSize: Number of entries to accumulate before GetIngotHashes returns (1000)
//   - logger: Structured logger
func NewIngotHashQueue(capacity, batchSize int, logger *slog.Logger) (*IngotHashQueue, error) {
	if capacity <= 0 {
		return nil, fmt.Errorf("capacity must be > 0, got %d", capacity)
	}
	if batchSize <= 0 || batchSize > capacity {
		return nil, fmt.Errorf("batchSize must be > 0 and <= capacity, got %d (capacity: %d)", batchSize, capacity)
	}

	q := &IngotHashQueue{
		entries:   make([]*models.IngotHashEntry, 0, capacity),
		capacity:  capacity,
		batchSize: batchSize,
		logger:    logger,
		metrics:   initIngotHashQueueMetrics(),
	}
	q.notFull = sync.NewCond(&q.mu)

	return q, nil
}

// initIngotHashQueueMetrics initializes Prometheus metrics
func initIngotHashQueueMetrics() *IngotHashQueueMetrics {
	metrics := &IngotHashQueueMetrics{
		QueueDepth: prometheus.NewGauge(prometheus.GaugeOpts{
			Name: "mint_ingot_hash_queue_depth",
			Help: "Current number of ingot hash entries in queue",
		}),
		EntriesAddedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingot_hash_entries_added_total",
			Help: "Total number of ingot hash entries added to queue",
		}),
		BatchesReadyTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingot_hash_batches_ready_total",
			Help: "Total number of 1000-entry batches ready for merkle tree building",
		}),
		ContextCancelled: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingot_hash_queue_context_cancelled_total",
			Help: "Total number of times GetIngotHashes was cancelled via context",
		}),
		AddBlockedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingot_hash_queue_add_blocked_total",
			Help: "Total number of times AddIngotHash blocked due to full queue",
		}),
		GetBlockedDuration: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_ingot_hash_queue_get_blocked_duration_seconds",
			Help:    "Duration GetIngotHashes was blocked waiting for batch",
			Buckets: prometheus.DefBuckets,
		}),
	}

	// Register metrics (ignore errors if already registered from tests)
	prometheus.Register(metrics.QueueDepth)
	prometheus.Register(metrics.EntriesAddedTotal)
	prometheus.Register(metrics.BatchesReadyTotal)
	prometheus.Register(metrics.ContextCancelled)
	prometheus.Register(metrics.AddBlockedTotal)
	prometheus.Register(metrics.GetBlockedDuration)

	return metrics
}

// AddIngotHash adds an ingot hash entry to the queue.
// Blocks if queue is at capacity (backpressure).
//
// Returns error only if queue is shutting down (via context).
func (q *IngotHashQueue) AddIngotHash(ctx context.Context, entry *models.IngotHashEntry) error {
	q.mu.Lock()
	defer q.mu.Unlock()

	// Block if queue is full (apply backpressure)
	for len(q.entries) >= q.capacity {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
			q.metrics.AddBlockedTotal.Inc()
			q.logger.Warn("ingot hash queue full, blocking AddIngotHash",
				"queue_size", len(q.entries),
				"capacity", q.capacity)
			q.notFull.Wait() // Wait for batch to be consumed
		}
	}

	// Add entry
	q.entries = append(q.entries, entry)
	q.metrics.EntriesAddedTotal.Inc()
	q.metrics.QueueDepth.Set(float64(len(q.entries)))

	// Log progress every 100 entries
	if len(q.entries)%100 == 0 {
		q.logger.Debug("ingot hash queue progress",
			"entries", len(q.entries),
			"progress_to_batch_percent", float64(len(q.entries)%q.batchSize)/float64(q.batchSize)*100)
	}

	// Signal if we've accumulated a full batch
	if len(q.entries) >= q.batchSize {
		q.notFull.Signal()
	}

	return nil
}

// GetIngotHashes blocks until batchSize entries are available, then returns them.
// Respects context cancellation for graceful shutdown.
//
// Returns:
//   - Exactly batchSize entries (1000) when successful
//   - Fewer entries only on context cancellation (graceful shutdown drain)
func (q *IngotHashQueue) GetIngotHashes(ctx context.Context) ([]*models.IngotHashEntry, error) {
	q.mu.Lock()
	defer q.mu.Unlock()

	startWait := prometheus.NewTimer(q.metrics.GetBlockedDuration)
	defer startWait.ObserveDuration()

	// Block until we have a full batch OR context is cancelled
	for len(q.entries) < q.batchSize {
		select {
		case <-ctx.Done():
			// Graceful shutdown: return whatever we have
			q.metrics.ContextCancelled.Inc()
			remaining := q.entries
			q.entries = make([]*models.IngotHashEntry, 0, q.capacity)
			q.metrics.QueueDepth.Set(0)

			if len(remaining) > 0 {
				q.logger.Info("ingot hash queue draining on shutdown",
					"remaining_entries", len(remaining))
				return remaining, nil
			}
			return nil, ctx.Err()

		default:
			q.notFull.Wait() // Block until batch ready
		}
	}

	// Extract exactly batchSize entries
	batch := q.entries[:q.batchSize]
	q.entries = q.entries[q.batchSize:]
	q.metrics.QueueDepth.Set(float64(len(q.entries)))
	q.metrics.BatchesReadyTotal.Inc()

	q.logger.Info("ingot hash batch ready",
		"batch_size", len(batch),
		"remaining_entries", len(q.entries))

	// Signal AddIngotHash that space is available
	q.notFull.Signal()

	return batch, nil
}

// Len returns the current number of entries in the queue (thread-safe)
func (q *IngotHashQueue) Len() int {
	q.mu.Lock()
	defer q.mu.Unlock()
	return len(q.entries)
}

// Drain returns all remaining entries and clears the queue.
// Used during graceful shutdown to flush incomplete batches.
func (q *IngotHashQueue) Drain() []*models.IngotHashEntry {
	q.mu.Lock()
	defer q.mu.Unlock()

	remaining := q.entries
	q.entries = make([]*models.IngotHashEntry, 0, q.capacity)
	q.metrics.QueueDepth.Set(0)

	if len(remaining) > 0 {
		q.logger.Info("ingot hash queue drained",
			"entries_drained", len(remaining))
	}

	return remaining
}
