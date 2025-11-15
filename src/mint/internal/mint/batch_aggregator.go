// Package mint provides the BatchAggregator component.
// BatchAggregator accumulates ingots into batches and triggers processing.
package mint

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
)

// ─────────────────────────────────────────────────────────────
// BatchAggregator Implementation
// ─────────────────────────────────────────────────────────────

// batchAggregator implements the BatchAggregator interface.
// It pulls ingots from the buffer, accumulates them to threshold (1000),
// and triggers MintEngine for batch processing.
type batchAggregator struct {
	buffer        IngotBuffer
	engine        MintEngine
	batchSize     int
	flushInterval time.Duration

	currentBatch []*TokenTorqIngot
	batchStarted time.Time
	mu           sync.Mutex // Protects currentBatch and batchStarted

	metrics *batchAggregatorMetrics
	logger  *slog.Logger
}

// batchAggregatorMetrics holds Prometheus metrics for BatchAggregator.
type batchAggregatorMetrics struct {
	batchesProcessed prometheus.Counter
	batchLatency     prometheus.Histogram
	partialFlushes   prometheus.Counter
	fullBatches      prometheus.Counter
	shutdownFlushes  prometheus.Counter
}

// NewBatchAggregator creates a new batch aggregator.
func NewBatchAggregator(
	buffer IngotBuffer,
	engine MintEngine,
	batchSize int,
	flushInterval time.Duration,
	logger *slog.Logger,
) BatchAggregator {
	metrics := &batchAggregatorMetrics{
		batchesProcessed: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_aggregator_batches_total",
			Help: "Total number of batches aggregated by BatchAggregator",
		}),
		batchLatency: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_aggregator_batch_latency_seconds",
			Help:    "Time from first ingot in batch to processing (seconds)",
			Buckets: []float64{0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0},
		}),
		partialFlushes: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_aggregator_partial_flushes_total",
			Help: "Total number of partial batch flushes (interval-based)",
		}),
		fullBatches: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_aggregator_full_batches_total",
			Help: "Total number of full batches (threshold-based)",
		}),
		shutdownFlushes: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_aggregator_shutdown_flushes_total",
			Help: "Total number of shutdown flushes",
		}),
	}

	// Register metrics
	prometheus.MustRegister(
		metrics.batchesProcessed,
		metrics.batchLatency,
		metrics.partialFlushes,
		metrics.fullBatches,
		metrics.shutdownFlushes,
	)

	return &batchAggregator{
		buffer:        buffer,
		engine:        engine,
		batchSize:     batchSize,
		flushInterval: flushInterval,
		currentBatch:  make([]*TokenTorqIngot, 0, batchSize),
		metrics:       metrics,
		logger:        logger,
	}
}

// ─────────────────────────────────────────────────────────────
// BatchAggregator Interface Implementation
// ─────────────────────────────────────────────────────────────

// Start begins the aggregation loop.
// Continuously pulls from buffer, builds batches, triggers MintEngine.
func (a *batchAggregator) Start(ctx context.Context) error {
	a.logger.Info("starting batch aggregator",
		"batch_size", a.batchSize,
		"flush_interval", a.flushInterval,
	)

	// Create ticker for interval-based flushes
	ticker := time.NewTicker(a.flushInterval)
	defer ticker.Stop()

	// Main loop: continuously pull ingots or handle timer/context
	for {
		// Create a short timeout context for Pop to avoid blocking forever
		popCtx, popCancel := context.WithTimeout(ctx, 50*time.Millisecond)
		ingot, err := a.buffer.Pop(popCtx)
		popCancel()

		if err == nil {
			// Got an ingot - add to batch
			a.mu.Lock()
			a.addIngotLocked(ingot)

			// Check if batch is full
			if len(a.currentBatch) >= a.batchSize {
				a.logger.Info("batch threshold reached",
					"batch_size", len(a.currentBatch),
				)
				a.flushBatchLocked(ctx, "threshold")
			}
			a.mu.Unlock()
			continue
		}

		// Check why Pop failed
		if ctx.Err() != nil {
			// Parent context cancelled - shutdown
			a.logger.Info("batch aggregator context cancelled, stopping")
			return ctx.Err()
		}

		if err == context.DeadlineExceeded {
			// Timeout - check ticker and continue
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-ticker.C:
				// Interval flush - process partial batch if exists
				a.mu.Lock()
				if len(a.currentBatch) > 0 {
					a.logger.Info("flush interval reached",
						"current_batch_size", len(a.currentBatch),
						"elapsed", time.Since(a.batchStarted),
					)
					a.flushBatchLocked(ctx, "interval")
				}
				a.mu.Unlock()
			default:
				// No ingot, no timer - continue polling
			}
			continue
		}

		// Other error
		a.logger.Error("failed to pop from buffer", "error", err)
	}
} // Flush immediately processes current batch (even if partial).
// Used during shutdown to prevent data loss.
func (a *batchAggregator) Flush() error {
	a.logger.Info("explicit flush requested")

	a.mu.Lock()
	defer a.mu.Unlock()

	if len(a.currentBatch) == 0 {
		a.logger.Info("no ingots to flush")
		return nil
	}

	ctx := context.Background()
	a.flushBatchLocked(ctx, "shutdown")

	return nil
}

// GetAccumulatedCount returns current batch size (for metrics/testing).
func (a *batchAggregator) GetAccumulatedCount() int {
	a.mu.Lock()
	defer a.mu.Unlock()
	return len(a.currentBatch)
}

// ─────────────────────────────────────────────────────────────
// Internal Methods (must be called with mu locked)
// ─────────────────────────────────────────────────────────────

// addIngotLocked adds an ingot to the current batch.
// Caller must hold a.mu.
func (a *batchAggregator) addIngotLocked(ingot *TokenTorqIngot) {
	// If this is first ingot in batch, record start time
	if len(a.currentBatch) == 0 {
		a.batchStarted = time.Now()
	}

	a.currentBatch = append(a.currentBatch, ingot)

	a.logger.Debug("ingot added to batch",
		"batch_size", len(a.currentBatch),
		"threshold", a.batchSize,
	)
}

// flushBatchLocked processes the current batch via MintEngine.
// Caller must hold a.mu.
func (a *batchAggregator) flushBatchLocked(ctx context.Context, reason string) {
	if len(a.currentBatch) == 0 {
		return
	}

	batchSize := len(a.currentBatch)
	latency := time.Since(a.batchStarted)

	a.logger.Info("flushing batch",
		"reason", reason,
		"batch_size", batchSize,
		"latency", latency,
	)

	// Process batch via MintEngine
	start := time.Now()
	err := a.engine.ProcessBatch(ctx, a.currentBatch)
	processingTime := time.Since(start)

	if err != nil {
		a.logger.Error("batch processing failed",
			"error", err,
			"batch_size", batchSize,
			"processing_time", processingTime,
		)
		// Note: In production, we might want to retry or queue failed batches
		// For now, we log and continue (ingots are lost on failure)
	} else {
		a.logger.Info("batch processed successfully",
			"batch_size", batchSize,
			"latency", latency,
			"processing_time", processingTime,
		)

		// Update metrics
		a.metrics.batchesProcessed.Inc()
		a.metrics.batchLatency.Observe(latency.Seconds())

		switch reason {
		case "threshold":
			a.metrics.fullBatches.Inc()
		case "interval":
			a.metrics.partialFlushes.Inc()
		case "shutdown":
			a.metrics.shutdownFlushes.Inc()
		}
	}

	// Reset batch (reuse slice capacity)
	a.currentBatch = a.currentBatch[:0]
}
