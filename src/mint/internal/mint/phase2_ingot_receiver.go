// internal/mint/phase2_ingot_receiver.go
// Phase 2 Ingot receiver - subscribes to NATS mint.ingots for hash-only ingots

package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"b2b/mint/internal/crypto"
	"b2b/mint/internal/models"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
)

// Phase2IngotReceiver subscribes to NATS and receives Phase2Ingots from Refinery
// This is the Phase 3 replacement for the old IngotReceiver that expected full units
type Phase2IngotReceiver struct {
	natsConn     *nats.Conn
	subscription *nats.Subscription
	queue        *IngotHashQueue
	ctx          context.Context
	logger       *slog.Logger
	metrics      *Phase2IngotMetrics
	verifier     *crypto.FalconVerifier // Phase 5: Signature verification
}

// Phase2IngotMetrics holds Prometheus metrics for Phase2Ingot reception
type Phase2IngotMetrics struct {
	IngotsReceived        prometheus.Counter
	ValidationErrors      *prometheus.CounterVec
	QueueErrors           prometheus.Counter
	IngotHashCountHist    prometheus.Histogram
	SignatureVerifyErrors prometheus.Counter   // Phase 5: Signature verification failures
	SignatureVerifyTime   prometheus.Histogram // Phase 5: Time to verify signatures
}

// NewPhase2IngotMetrics creates and registers Phase 2 ingot metrics
func NewPhase2IngotMetrics() *Phase2IngotMetrics {
	metrics := &Phase2IngotMetrics{
		IngotsReceived: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase2_ingots_received_total",
			Help: "Total Phase 2 ingots received from NATS",
		}),
		ValidationErrors: prometheus.NewCounterVec(prometheus.CounterOpts{
			Name: "mint_ingot_validation_errors_total",
			Help: "Ingot validation errors by reason",
		}, []string{"reason"}),
		QueueErrors: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingot_queue_errors_total",
			Help: "Errors queuing ingot hashes",
		}),
		IngotHashCountHist: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_ingot_hash_count",
			Help:    "Distribution of hash counts in received ingots",
			Buckets: []float64{100, 500, 1000, 2000, 3600, 5000, 10000},
		}),
		SignatureVerifyErrors: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase2_signature_verification_errors_total",
			Help: "Total Falcon-1024 signature verification failures",
		}),
		SignatureVerifyTime: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase2_signature_verify_duration_seconds",
			Help:    "Time taken to verify Falcon-1024 signatures on Phase2Ingots",
			Buckets: prometheus.DefBuckets,
		}),
	}

	// Try to register metrics (ignore if already registered in tests)
	prometheus.Register(metrics.IngotsReceived)
	prometheus.Register(metrics.ValidationErrors)
	prometheus.Register(metrics.QueueErrors)
	prometheus.Register(metrics.IngotHashCountHist)
	prometheus.Register(metrics.SignatureVerifyErrors)
	prometheus.Register(metrics.SignatureVerifyTime)

	return metrics
}

// NewPhase2IngotReceiver creates a new Phase2IngotReceiver
func NewPhase2IngotReceiver(nc *nats.Conn, queue *IngotHashQueue, ctx context.Context, logger *slog.Logger) (*Phase2IngotReceiver, error) {
	if nc == nil {
		return nil, fmt.Errorf("NATS connection cannot be nil")
	}
	if queue == nil {
		return nil, fmt.Errorf("IngotHashQueue cannot be nil")
	}

	metrics := NewPhase2IngotMetrics()

	verifier := crypto.NewFalconVerifier()

	return &Phase2IngotReceiver{
		natsConn: nc,
		queue:    queue,
		ctx:      ctx,
		logger:   logger,
		metrics:  metrics,
		verifier: verifier,
	}, nil
}

// Start subscribes to mint.phase2.ingots and begins processing Phase2Ingots
func (pir *Phase2IngotReceiver) Start() error {
	sub, err := pir.natsConn.Subscribe("mint.phase2.ingots", pir.handleIngot)
	if err != nil {
		return fmt.Errorf("failed to subscribe to mint.phase2.ingots: %w", err)
	}

	pir.subscription = sub
	pir.logger.Info("Phase2IngotReceiver started",
		"subject", "mint.phase2.ingots",
		"status", "listening")

	return nil
}

// handleIngot processes incoming Phase2Ingot batch messages from NATS
func (pir *Phase2IngotReceiver) handleIngot(msg *nats.Msg) {
	// First, try to unmarshal as a batch wrapper
	var batch struct {
		BatchID   string                `json:"batch_id"`
		Timestamp string                `json:"timestamp"`
		Count     int                   `json:"count"`
		Ingots    []*models.Phase2Ingot `json:"ingots"`
	}

	if err := json.Unmarshal(msg.Data, &batch); err != nil {
		pir.logger.Error("failed to unmarshal Phase2 batch",
			"error", err,
			"msg_size", len(msg.Data))
		pir.metrics.ValidationErrors.WithLabelValues("unmarshal").Inc()
		return
	}

	pir.logger.Info("received NATS batch",
		"batch_id", batch.BatchID,
		"count", batch.Count,
		"ingots", len(batch.Ingots),
		"timestamp", batch.Timestamp)

	// Process each ingot in the batch
	successCount := 0
	failCount := 0

	for i, ingot := range batch.Ingots {
		// Validate ingot structure
		if err := ingot.Validate(); err != nil {
			pir.logger.Error("ingot validation failed",
				"error", err,
				"ingot_id", ingot.ID,
				"branch_hash", truncateHash(ingot.BranchHash),
				"hash_count", ingot.HashCount)
			pir.metrics.ValidationErrors.WithLabelValues("validation").Inc()
			failCount++
			continue
		}

		// Phase 5: Verify Falcon-1024 signature from Refinery
		verifyStart := time.Now()
		if err := pir.verifier.VerifyPhase2Ingot(
			ingot.ID,
			ingot.BranchHash,
			ingot.HashCount,
			ingot.Timestamp.Format(time.RFC3339),
			ingot.Signature,
			ingot.PublicKey,
		); err != nil {
			pir.logger.Error("Falcon-1024 signature verification FAILED - REJECTING INGOT",
				"error", err,
				"ingot_id", ingot.ID,
				"branch_hash", truncateHash(ingot.BranchHash),
				"signature_len", len(ingot.Signature),
				"pubkey_len", len(ingot.PublicKey))
			pir.metrics.SignatureVerifyErrors.Inc()
			pir.metrics.ValidationErrors.WithLabelValues("signature").Inc()
			failCount++
			continue
		}
		verifyTime := time.Since(verifyStart)
		pir.metrics.SignatureVerifyTime.Observe(verifyTime.Seconds())

		pir.logger.Info("Falcon-1024 signature VERIFIED ✓",
			"ingot_id", ingot.ID,
			"verify_time_ms", verifyTime.Milliseconds(),
			"refinery_pubkey", truncateHash(ingot.PublicKey))

		// Record hash count distribution
		pir.metrics.IngotHashCountHist.Observe(float64(ingot.HashCount))

		// Convert to IngotHashEntry and add to queue
		hashEntry := models.NewIngotHashEntry(ingot)
		if err := pir.queue.AddIngotHash(pir.ctx, hashEntry); err != nil {
			pir.logger.Error("failed to process ingot from NATS batch",
				"error", err,
				"batch_id", batch.BatchID,
				"ingot_index", i,
				"ingot_id", ingot.ID)
			pir.metrics.QueueErrors.Inc()
			failCount++
			continue
		}

		// Success
		pir.metrics.IngotsReceived.Inc()
		successCount++
		pir.logger.Info("Phase 2 ingot received",
			"ingot_id", ingot.ID,
			"branch_hash", truncateHash(ingot.BranchHash),
			"hash_count", ingot.HashCount,
			"contracts", len(ingot.ContractIDs),
			"diggers", len(ingot.DiggerIDs),
			"queue_depth", pir.queue.Len())
	}

	pir.logger.Info("processed NATS batch",
		"batch_id", batch.BatchID,
		"total", batch.Count,
		"success", successCount,
		"failed", failCount)
}

// Stop unsubscribes from NATS and stops receiving ingots
func (pir *Phase2IngotReceiver) Stop() error {
	if pir.subscription != nil {
		if err := pir.subscription.Unsubscribe(); err != nil {
			return fmt.Errorf("failed to unsubscribe: %w", err)
		}
		pir.logger.Info("Phase2IngotReceiver stopped")
	}
	return nil
}

// truncateHash returns first 16 chars of hash for logging (e.g., "a1b2c3d4e5f6g7h8...")
func truncateHash(hash string) string {
	if len(hash) > 16 {
		return hash[:16] + "..."
	}
	return hash
}
