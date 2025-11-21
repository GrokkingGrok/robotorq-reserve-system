// internal/refinery/hash_batch_receiver.go
// Receives and verifies hash-only ore batches from Digger (Phase 5)

package refinery

import (
	"fmt"
	"log/slog"
	"sync"

	"b2b/refinery/internal/crypto"
	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// Metrics for hash batch processing
var (
	hashBatchesReceivedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_hash_batches_received_total",
		Help: "Total number of hash batches received from Digger",
	})
	signaturesVerifiedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_falcon_signatures_verified_total",
		Help: "Total number of Falcon-1024 signatures successfully verified",
	})
	signaturesFailedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_falcon_signatures_failed_total",
		Help: "Total number of Falcon-1024 signature verification failures",
	})
	hashesReceivedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_jtu_hashes_received_total",
		Help: "Total number of JTU hashes received in batches",
	})
	verificationDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_falcon_verification_duration_seconds",
		Help:    "Time taken to verify Falcon-1024 signatures",
		Buckets: prometheus.ExponentialBuckets(0.001, 2, 10), // 1ms to 1s
	})

	hashBatchMetricsOnce sync.Once
)

func init() {
	// Register hash batch metrics (only once)
	hashBatchMetricsOnce.Do(func() {
		prometheus.MustRegister(
			hashBatchesReceivedTotal,
			signaturesVerifiedTotal,
			signaturesFailedTotal,
			hashesReceivedTotal,
			verificationDuration,
		)
	})
}

// HashBatchReceiver handles incoming hash batches with Falcon-1024 verification
type HashBatchReceiver struct {
	verifier         *crypto.FalconVerifier
	queueMgr         *QueueManager
	skipVerification bool // For testing - skip Falcon verification
}

// NewHashBatchReceiver creates a new hash batch receiver with verification
func NewHashBatchReceiver(queueMgr *QueueManager, skipVerification bool) *HashBatchReceiver {
	return &HashBatchReceiver{
		verifier:         crypto.NewFalconVerifier(),
		queueMgr:         queueMgr,
		skipVerification: skipVerification,
	}
}

// ReceiveHashBatch processes a hash batch with Falcon-1024 signature verification
func (hbr *HashBatchReceiver) ReceiveHashBatch(batch *models.HashBatchOre) error {
	hashBatchesReceivedTotal.Inc()

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
		timer := prometheus.NewTimer(verificationDuration)
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
			signaturesFailedTotal.Inc()

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

		signaturesVerifiedTotal.Inc()

		slog.Info("✅ Falcon signature verified",
			"contract_id", batch.ContractID,
			"digger_id", batch.DiggerID,
			"hashes", batch.HashCount)
	}

	hashesReceivedTotal.Add(float64(batch.HashCount))

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
