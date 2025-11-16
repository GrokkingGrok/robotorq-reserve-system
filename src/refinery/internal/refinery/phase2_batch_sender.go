// internal/refinery/phase2_batch_sender.go
// Phase 2: Time-based batch sender for Phase2Ingots to Mint

package refinery

import (
	"context"
	"log/slog"
	"time"

	"github.com/prometheus/client_golang/prometheus"
)

var (
	// phase2BatchesSentTotal tracks total Phase2 batches sent to Mint
	phase2BatchesSentTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_phase2_batches_sent_total",
		Help: "Total Phase2 batches of ingots sent to Mint",
	})

	// phase2IngotsSentTotal tracks total individual Phase2 ingots sent
	phase2IngotsSentTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_phase2_ingots_sent_total",
		Help: "Total individual Phase2 ingots sent to Mint",
	})

	// phase2BatchSendDuration tracks time to send Phase2 batches
	phase2BatchSendDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_phase2_batch_send_duration_seconds",
		Help:    "Time taken to send a Phase2 batch to Mint",
		Buckets: prometheus.DefBuckets,
	})

	// phase2BatchSizeGauge tracks the size of Phase2 batches being sent
	phase2BatchSizeGauge = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_phase2_batch_size",
		Help:    "Number of Phase2 ingots in each batch sent to Mint",
		Buckets: []float64{1, 5, 10, 20, 50, 100},
	})
)

func init() {
	prometheus.MustRegister(phase2BatchesSentTotal)
	prometheus.MustRegister(phase2IngotsSentTotal)
	prometheus.MustRegister(phase2BatchSendDuration)
	prometheus.MustRegister(phase2BatchSizeGauge)
}

// Phase2MintPublisher defines the interface for sending Phase2 ingots to Mint
type Phase2MintPublisher interface {
	PublishPhase2Batch(ingots []*Phase2Ingot) error
}

// Phase2BatchSender periodically sends completed Phase2 ingots to Mint
type Phase2BatchSender struct {
	assembler     *Phase2IngotAssembler
	publisher     Phase2MintPublisher
	batchInterval time.Duration
	ctx           context.Context
}

// NewPhase2BatchSender creates a new Phase2 batch sender
func NewPhase2BatchSender(
	ctx context.Context,
	assembler *Phase2IngotAssembler,
	publisher Phase2MintPublisher,
	batchInterval time.Duration,
) *Phase2BatchSender {
	return &Phase2BatchSender{
		assembler:     assembler,
		publisher:     publisher,
		batchInterval: batchInterval,
		ctx:           ctx,
	}
}

// Start begins the batch sending process (blocking goroutine)
func (bs *Phase2BatchSender) Start() {
	slog.Info("starting Phase2 batch sender",
		"interval", bs.batchInterval,
	)

	ticker := time.NewTicker(bs.batchInterval)
	defer ticker.Stop()

	for {
		select {
		case <-bs.ctx.Done():
			slog.Info("Phase2 batch sender shutting down")
			// Send any remaining ingots before shutdown
			bs.sendBatch()
			return

		case <-ticker.C:
			// Timer fired - send batch
			bs.sendBatch()
		}
	}
}

// sendBatch collects completed Phase2 ingots and sends them to Mint
func (bs *Phase2BatchSender) sendBatch() {
	// Get all completed ingots from assembler
	ingots := bs.assembler.GetCompletedIngots()

	// Skip if no ingots to send
	if len(ingots) == 0 {
		slog.Debug("no Phase2 ingots to send in this batch")
		return
	}

	timer := prometheus.NewTimer(phase2BatchSendDuration)
	defer timer.ObserveDuration()

	slog.Info("sending Phase2 batch to mint",
		"batch_size", len(ingots),
		"interval", bs.batchInterval,
	)

	// Publish to Mint via NATS
	if err := bs.publisher.PublishPhase2Batch(ingots); err != nil {
		slog.Error("failed to publish Phase2 batch to mint",
			"error", err,
			"batch_size", len(ingots),
		)
		// TODO: Consider retry logic or dead letter queue
		return
	}

	// Update metrics
	phase2BatchesSentTotal.Inc()
	phase2IngotsSentTotal.Add(float64(len(ingots)))
	phase2BatchSizeGauge.Observe(float64(len(ingots)))

	slog.Info("Phase2 batch sent successfully",
		"batch_size", len(ingots),
	)

	// Log individual ingot IDs for traceability
	for i, ingot := range ingots {
		slog.Debug("Phase2 ingot sent",
			"batch_index", i,
			"ingot_id", ingot.ID,
			"branch_hash", ingot.BranchHash,
			"hash_count", ingot.HashCount,
			"contracts", len(ingot.ContractIDs),
		)
	}
}
