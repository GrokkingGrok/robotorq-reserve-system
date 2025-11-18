// Package mint provides the core minting engine for TokenTorq batch processing.
package mint

import (
	"context"
	"fmt"
	"log/slog"
	"sync/atomic"
	"time"

	"github.com/google/uuid"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// mintEngine coordinates batch processing and event generation.
//
// Architecture:
//  1. Receive batch from BatchAggregator
//  2. Use pluggable BatchHasher to compute hash + totals
//  3. Generate MintEvent with batch metadata
//  4. Publish event via DistoDamClient to NATS
//  5. Update cumulative metrics
//
// Design principle: BatchHasher is pluggable.
// Current: SimpleBatchHasher (SHA256)
// Future: FullMerkleBuilder (Merkle tree with proofs)
type mintEngine struct {
	hasher BatchHasher    // Pluggable hash implementation
	client DistoDamClient // NATS publisher
	logger *slog.Logger   // Structured logging

	// Cumulative metrics (atomic for thread safety)
	totalProcessed atomic.Int64  // Total ingots processed
	totalRobo      atomic.Uint64 // Total RoboTorq aggregated (stored as uint64, interpreted as float64)

	// Prometheus metrics
	batchesProcessed   prometheus.Counter
	ingotsProcessed    prometheus.Counter
	roboAggregated     prometheus.Counter
	processingDuration prometheus.Histogram
	publishFailures    prometheus.Counter
}

// NewMintEngine creates a new MintEngine instance.
//
// Parameters:
//   - hasher: BatchHasher implementation (SimpleBatchHasher or FullMerkleBuilder)
//   - client: DistoDamClient for NATS publishing
//   - logger: Structured logger for operations
//
// The hasher is pluggable to support future upgrades:
//
//	hasher := NewSimpleBatchHasher()           // v0: SHA256
//	hasher := NewFullMerkleBuilder(SHA256)     // v1: Merkle tree
func NewMintEngine(hasher BatchHasher, client DistoDamClient, logger *slog.Logger) MintEngine {
	engine := &mintEngine{
		hasher: hasher,
		client: client,
		logger: logger,
	}

	// Initialize Prometheus metrics
	engine.batchesProcessed = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_batches_processed_total",
		Help: "Total number of batches processed by MintEngine",
	})

	engine.ingotsProcessed = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_ingots_processed_total",
		Help: "Total number of ingots processed across all batches",
	})

	engine.roboAggregated = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_robo_aggregated_total",
		Help: "Total RoboTorq aggregated across all batches",
	})

	engine.processingDuration = promauto.NewHistogram(prometheus.HistogramOpts{
		Name:    "mint_batch_processing_seconds",
		Help:    "Time spent processing batches (hashing + publishing)",
		Buckets: prometheus.DefBuckets,
	})

	engine.publishFailures = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_publish_failures_total",
		Help: "Total number of failed NATS publish attempts",
	})

	return engine
}

// ProcessBatch processes a batch of ingots and publishes a MintEvent.
//
// PHASE 6 UPDATE: Now sends individual IngotStakes[] instead of summing.
//
// Flow:
//  1. Use BatchHasher to compute hash + individual stakes
//  2. Generate MintEvent with IngotStakes array
//  3. Publish to DistoDam via NATS (mint.batches topic)
//  4. Update cumulative metrics (sum from stakes for backward compat)
//  5. Log structured event
//
// Error handling:
//   - Hasher errors: logged and returned (caller should retry)
//   - Publish errors: logged, metric incremented, returned (caller decides retry)
func (e *mintEngine) ProcessBatch(ctx context.Context, batch []*TokenTorqIngot) error {
	startTime := time.Now()

	// Validate batch
	if len(batch) == 0 {
		return fmt.Errorf("cannot process empty batch")
	}

	// Use BatchHasher to compute hash + individual stakes
	batchHash, ingotStakes, err := e.hasher.Hash(batch)
	if err != nil {
		e.logger.Error("batch hashing failed",
			"error", err,
			"batch_size", len(batch),
		)
		return fmt.Errorf("batch hashing failed: %w", err)
	}

	// Calculate total for metrics (backward compatibility)
	var totalRobo float64
	for _, stake := range ingotStakes {
		totalRobo += stake.RoboStakeTotal
	}

	// Generate MintEvent (PHASE 6: Now includes IngotStakes array)
	event := &MintEvent{
		BatchHash:       batchHash,
		IngotStakes:     ingotStakes, // NEW: Individual stakes, not summed
		IngotsProcessed: len(batch),
		BatchID:         uuid.New().String(),
		Timestamp:       time.Now().UTC(),
		// DEPRECATED (kept for backward compat - will be removed in Phase 7)
		TotalRoboTorq: totalRobo,
		SaleValueUSD:  0, // No longer calculated
	}

	// Publish to DistoDam via NATS
	if err := e.client.Publish(ctx, event); err != nil {
		e.publishFailures.Inc()
		e.logger.Error("failed to publish MintEvent",
			"error", err,
			"batch_id", event.BatchID,
			"batch_hash", event.BatchHash,
			"ingots", event.IngotsProcessed,
			"stakes_count", len(event.IngotStakes),
		)
		return fmt.Errorf("failed to publish MintEvent: %w", err)
	}

	// Update cumulative metrics (atomic operations)
	e.totalProcessed.Add(int64(len(batch)))
	e.totalRobo.Add(floatToUint64(totalRobo))

	// Update Prometheus metrics
	e.batchesProcessed.Inc()
	e.ingotsProcessed.Add(float64(len(batch)))
	e.roboAggregated.Add(totalRobo)
	e.processingDuration.Observe(time.Since(startTime).Seconds())

	// Log successful processing
	e.logger.Info("batch processed and published",
		"batch_id", event.BatchID,
		"batch_hash", event.BatchHash,
		"ingots_count", event.IngotsProcessed,
		"ingot_stakes_count", len(event.IngotStakes),
		"total_robo", totalRobo,
		"duration_ms", time.Since(startTime).Milliseconds(),
	)

	return nil
}

// GetTotalProcessed returns cumulative ingots processed.
// Thread-safe via atomic operations.
func (e *mintEngine) GetTotalProcessed() int64 {
	return e.totalProcessed.Load()
}

// GetTotalRoboAggregated returns cumulative RoboTorq aggregated.
// Thread-safe via atomic operations.
func (e *mintEngine) GetTotalRoboAggregated() float64 {
	return uint64ToFloat(e.totalRobo.Load())
}

// ─────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────

// floatToUint64 converts float64 to uint64 for atomic storage.
// Uses simple multiplication by 100 to preserve 2 decimal places.
func floatToUint64(f float64) uint64 {
	return uint64(f * 100)
}

// uint64ToFloat converts uint64 back to float64.
func uint64ToFloat(u uint64) float64 {
	return float64(u) / 100.0
}
