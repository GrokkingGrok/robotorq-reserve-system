// internal/refinery/hash_batch_receiver.go
// Receives and verifies hash-only ore batches from Digger (Phase 5)

package refinery

import (
	"fmt"
	"log/slog"

	"b2b/refinery/internal/crypto"
	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// HashBatchReceiver handles incoming hash batches with Falcon-1024 verification
type HashBatchReceiver struct {
	verifier         *crypto.FalconVerifier
	queueMgr         *QueueManager
	metrics          *HashBatchMetrics
	skipVerification bool // For testing - skip Falcon verification
}

// HashBatchMetrics tracks signature verification metrics
type HashBatchMetrics struct {
	BatchesReceivedTotal    prometheus.Counter
	SignaturesVerifiedTotal prometheus.Counter
	SignaturesFailedTotal   prometheus.Counter
	HashesReceivedTotal     prometheus.Counter
	VerificationDuration    prometheus.Histogram
}

// NewHashBatchMetrics creates Prometheus metrics for hash batch processing
func NewHashBatchMetrics() *HashBatchMetrics {
	return &HashBatchMetrics{
		BatchesReceivedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_hash_batches_received_total",
			Help: "Total number of hash batches received from Digger",
		}),
		SignaturesVerifiedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_falcon_signatures_verified_total",
			Help: "Total number of Falcon-1024 signatures successfully verified",
		}),
		SignaturesFailedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_falcon_signatures_failed_total",
			Help: "Total number of Falcon-1024 signature verification failures",
		}),
		HashesReceivedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_jtu_hashes_received_total",
			Help: "Total number of JTU hashes received in batches",
		}),
		VerificationDuration: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "refinery_falcon_verification_duration_seconds",
			Help:    "Time taken to verify Falcon-1024 signatures",
			Buckets: prometheus.ExponentialBuckets(0.001, 2, 10), // 1ms to 1s
		}),
	}
}

// RegisterMetrics registers all metrics with Prometheus
func (m *HashBatchMetrics) RegisterMetrics() {
	prometheus.MustRegister(
		m.BatchesReceivedTotal,
		m.SignaturesVerifiedTotal,
		m.SignaturesFailedTotal,
		m.HashesReceivedTotal,
		m.VerificationDuration,
	)
}

// NewHashBatchReceiver creates a new hash batch receiver with verification
func NewHashBatchReceiver(queueMgr *QueueManager, skipVerification bool) *HashBatchReceiver {
	metrics := NewHashBatchMetrics()
	metrics.RegisterMetrics()

	return &HashBatchReceiver{
		verifier:         crypto.NewFalconVerifier(),
		queueMgr:         queueMgr,
		metrics:          metrics,
		skipVerification: skipVerification,
	}
}

// ReceiveHashBatch processes a hash batch with Falcon-1024 signature verification
func (hbr *HashBatchReceiver) ReceiveHashBatch(batch *models.HashBatchOre) error {
	hbr.metrics.BatchesReceivedTotal.Inc()

	// 1. Validate structure
	if err := batch.Validate(); err != nil {
		slog.Error("hash batch validation failed",
			"contract", batch.ContractID,
			"digger", batch.DiggerID,
			"error", err)
		return fmt.Errorf("validation failed: %w", err)
	}

	slog.Info("hash batch received",
		"contract", batch.ContractID,
		"digger", batch.DiggerID,
		"hashes", batch.HashCount,
		"timestamp", batch.Timestamp)

	// 2. Verify Falcon-1024 signature (Phase 5 - CRITICAL SECURITY CHECK)
	if hbr.skipVerification {
		slog.Warn("⚠️  SKIPPING Falcon verification (testing mode)",
			"contract", batch.ContractID,
			"digger", batch.DiggerID)
	} else {
		timer := prometheus.NewTimer(hbr.metrics.VerificationDuration)
		defer timer.ObserveDuration()

		err := hbr.verifier.VerifyHashBatch(
			batch.ContractID,
			batch.DiggerID,
			batch.MilestoneIndex,
			batch.Joules,
			batch.RoboStake,
			batch.Hashes,
			batch.Timestamp,
			batch.Signature,
			batch.PublicKey,
		)

		if err != nil {
			hbr.metrics.SignaturesFailedTotal.Inc()

			slog.Error("SECURITY: Falcon signature verification FAILED",
				"contract", batch.ContractID,
				"digger", batch.DiggerID,
				"hashes", batch.HashCount,
				"error", err,
				"action", "REJECTED")

			// TODO Phase 5: Implement slashing for invalid signatures
			// - Record violation in Trust service
			// - Slash digger's stake (10% for first offense)
			// - Ban digger if repeated offenses

			return fmt.Errorf("signature verification failed: %w", err)
		}

		hbr.metrics.SignaturesVerifiedTotal.Inc()

		slog.Info("✅ Falcon signature verified",
			"contract_id", batch.ContractID,
			"digger_id", batch.DiggerID,
			"hashes", batch.HashCount)
	}

	hbr.metrics.HashesReceivedTotal.Add(float64(batch.HashCount))

	// 3. Distribute RoboStake across all hashes
	roboStakePerHash := batch.RoboStake / float64(batch.HashCount)

	// 4. Process verified hashes - add to queue for Phase2 ingot assembly
	for _, hash := range batch.Hashes {
		if err := hbr.queueMgr.AddHash(hash, batch.ContractID, batch.DiggerID, roboStakePerHash); err != nil {
			slog.Warn("failed to queue hash after verification",
				"contract", batch.ContractID,
				"digger", batch.DiggerID,
				"hash", hash[:16]+"...",
				"error", err)
			// Continue processing other hashes
		}
	}

	slog.Info("verified hashes queued for ingot assembly",
		"contract_id", batch.ContractID,
		"digger_id", batch.DiggerID,
		"hashes_queued", batch.HashCount)

	return nil
}
