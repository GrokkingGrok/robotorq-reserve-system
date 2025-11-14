// internal/refinery/batch_sender.go
// Time-based batch sender for TokenTorq Ingots to Mint

package refinery

import (
	"context"
	"log/slog"
	"time"

	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

var (
	// batchesSentTotal tracks total batches sent to Mint
	batchesSentTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_batches_sent_total",
		Help: "Total batches of ingots sent to Mint",
	})

	// ingotsSentTotal tracks total individual ingots sent
	ingotsSentTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_ingots_sent_total",
		Help: "Total individual ingots sent to Mint",
	})

	// batchSendDuration tracks time to send batches
	batchSendDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_batch_send_duration_seconds",
		Help:    "Time taken to send a batch to Mint",
		Buckets: prometheus.DefBuckets,
	})

	// batchSizeGauge tracks the size of batches being sent
	batchSizeGauge = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_batch_size",
		Help:    "Number of ingots in each batch sent to Mint",
		Buckets: []float64{1, 5, 10, 20, 50, 100},
	})
)

func init() {
	prometheus.MustRegister(batchesSentTotal)
	prometheus.MustRegister(ingotsSentTotal)
	prometheus.MustRegister(batchSendDuration)
	prometheus.MustRegister(batchSizeGauge)
}

// MintPublisher defines the interface for sending ingots to Mint
type MintPublisher interface {
	PublishBatch(ingots []*models.TokenTorqIngot) error
}

// BatchSender periodically sends completed ingots to Mint
type BatchSender struct {
	assembler     *IngotAssembler
	publisher     MintPublisher
	batchInterval time.Duration
	ctx           context.Context
}

// NewBatchSender creates a new batch sender
func NewBatchSender(
	ctx context.Context,
	assembler *IngotAssembler,
	publisher MintPublisher,
	batchInterval time.Duration,
) *BatchSender {
	return &BatchSender{
		assembler:     assembler,
		publisher:     publisher,
		batchInterval: batchInterval,
		ctx:           ctx,
	}
}

// Start begins the batch sending process (blocking goroutine)
func (bs *BatchSender) Start() {
	slog.Info("starting batch sender",
		"interval", bs.batchInterval,
	)

	ticker := time.NewTicker(bs.batchInterval)
	defer ticker.Stop()

	for {
		select {
		case <-bs.ctx.Done():
			slog.Info("batch sender shutting down")
			// Send any remaining ingots before shutdown
			bs.sendBatch()
			return

		case <-ticker.C:
			// Timer fired - send batch
			bs.sendBatch()
		}
	}
}

// sendBatch collects completed ingots and sends them to Mint
func (bs *BatchSender) sendBatch() {
	// Get all completed ingots from assembler
	ingots := bs.assembler.GetCompletedIngots()

	// Skip if no ingots to send
	if len(ingots) == 0 {
		slog.Debug("no ingots to send in this batch")
		return
	}

	timer := prometheus.NewTimer(batchSendDuration)
	defer timer.ObserveDuration()

	slog.Info("sending batch to mint",
		"batch_size", len(ingots),
		"interval", bs.batchInterval,
	)

	// Publish to Mint via NATS
	if err := bs.publisher.PublishBatch(ingots); err != nil {
		slog.Error("failed to publish batch to mint",
			"error", err,
			"batch_size", len(ingots),
		)
		// TODO: Consider retry logic or dead letter queue
		return
	}

	// Update metrics
	batchesSentTotal.Inc()
	ingotsSentTotal.Add(float64(len(ingots)))
	batchSizeGauge.Observe(float64(len(ingots)))

	slog.Info("batch sent successfully",
		"batch_size", len(ingots),
		"total_batches", batchesSentTotal,
	)

	// Log individual ingot IDs for traceability
	for i, ingot := range ingots {
		slog.Debug("ingot sent",
			"batch_index", i,
			"ingot_id", ingot.IngotID,
			"joules", ingot.JouleTorqTotal,
			"contracts", len(ingot.ContractIDs),
		)
	}
}
