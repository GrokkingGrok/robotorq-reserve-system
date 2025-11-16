// internal/mint/phase2_ingot_receiver.go
// Phase 2 Ingot receiver - subscribes to NATS mint.ingots for hash-only ingots

package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"

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
}

// Phase2IngotMetrics holds Prometheus metrics for Phase2Ingot reception
type Phase2IngotMetrics struct {
	IngotsReceived     prometheus.Counter
	ValidationErrors   *prometheus.CounterVec
	QueueErrors        prometheus.Counter
	IngotHashCountHist prometheus.Histogram
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
	}

	// Try to register metrics (ignore if already registered in tests)
	prometheus.Register(metrics.IngotsReceived)
	prometheus.Register(metrics.ValidationErrors)
	prometheus.Register(metrics.QueueErrors)
	prometheus.Register(metrics.IngotHashCountHist)

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

	return &Phase2IngotReceiver{
		natsConn: nc,
		queue:    queue,
		ctx:      ctx,
		logger:   logger,
		metrics:  metrics,
	}, nil
}

// Start subscribes to mint.ingots and begins processing Phase2Ingots
func (pir *Phase2IngotReceiver) Start() error {
	sub, err := pir.natsConn.Subscribe("mint.ingots", pir.handleIngot)
	if err != nil {
		return fmt.Errorf("failed to subscribe to mint.ingots: %w", err)
	}

	pir.subscription = sub
	pir.logger.Info("Phase2IngotReceiver started",
		"subject", "mint.ingots",
		"status", "listening")

	return nil
}

// handleIngot processes incoming Phase2Ingot messages from NATS
func (pir *Phase2IngotReceiver) handleIngot(msg *nats.Msg) {
	// Unmarshal JSON
	var ingot models.Phase2Ingot
	if err := json.Unmarshal(msg.Data, &ingot); err != nil {
		pir.logger.Error("failed to unmarshal Phase2Ingot",
			"error", err,
			"msg_size", len(msg.Data))
		pir.metrics.ValidationErrors.WithLabelValues("unmarshal").Inc()
		return
	}

	// Validate ingot structure
	if err := ingot.Validate(); err != nil {
		pir.logger.Warn("invalid Phase2Ingot received",
			"error", err,
			"ingot_id", ingot.ID,
			"branch_hash", truncateHash(ingot.BranchHash),
			"hash_count", ingot.HashCount)
		pir.metrics.ValidationErrors.WithLabelValues("validation").Inc()
		return
	}

	// Record hash count distribution
	pir.metrics.IngotHashCountHist.Observe(float64(ingot.HashCount))

	// Convert to IngotHashEntry and add to queue
	hashEntry := models.NewIngotHashEntry(&ingot)
	if err := pir.queue.AddIngotHash(pir.ctx, hashEntry); err != nil {
		pir.logger.Error("failed to queue ingot hash",
			"error", err,
			"ingot_id", ingot.ID,
			"branch_hash", truncateHash(ingot.BranchHash))
		pir.metrics.QueueErrors.Inc()
		return
	}

	// Success
	pir.metrics.IngotsReceived.Inc()
	pir.logger.Info("Phase 2 ingot received",
		"ingot_id", ingot.ID,
		"branch_hash", truncateHash(ingot.BranchHash),
		"hash_count", ingot.HashCount,
		"contracts", len(ingot.ContractIDs),
		"diggers", len(ingot.DiggerIDs),
		"queue_depth", pir.queue.Len())
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
