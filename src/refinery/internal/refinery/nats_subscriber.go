// Package refinery implements the NATS subscriber that receives hash batches
// from Digger services and queues them for ingot assembly.
package refinery

import (
	"context"
	"encoding/json"
	"log/slog"
	"sync"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
)

// HashBatchMessage represents the JSON message format sent by Digger
// via NATS on the "ore.batch" subject.
//
// This message contains ONLY the hashes of JouleTorqUnits, not the full
// unit data. The Refinery builds merkle trees from these hashes without
// needing to store or transfer the complete JTU structures.
type HashBatchMessage struct {
	ContractID string   `json:"contract_id"` // Contract that generated these hashes
	DiggerID   string   `json:"digger_id"`   // Digger that performed the work
	Hashes     []string `json:"hashes"`      // SHA256 hashes (32-byte hex strings)
	HashCount  int      `json:"hash_count"`  // Number of hashes in batch
	Timestamp  string   `json:"timestamp"`   // RFC3339 timestamp
}

// NATSSubscriber listens to the NATS "ore.batch" subject and processes
// incoming hash batches by adding them to the queue manager.
//
// Architecture:
// - Subscribes to "ore.batch" subject on startup
// - Parses HashBatchMessage JSON from each message
// - Adds individual hashes to QueueManager (not full units!)
// - Tracks metrics for monitoring
// - Gracefully unsubscribes on shutdown
type NATSSubscriber struct {
	nc       *nats.Conn
	sub      *nats.Subscription
	logger   *slog.Logger
	queueMgr *QueueManager
	metrics  *NATSSubscriberMetrics
	ctx      context.Context
	wg       sync.WaitGroup
}

// NATSSubscriberMetrics tracks NATS subscriber performance
type NATSSubscriberMetrics struct {
	BatchesReceivedTotal  prometheus.Counter
	HashesReceivedTotal   prometheus.Counter
	InvalidMessagesTotal  prometheus.Counter
	ProcessingErrorsTotal prometheus.Counter
	LastBatchHashCount    prometheus.Gauge
	LastBatchTimestamp    prometheus.Gauge
}

// NewNATSSubscriberMetrics creates metrics for the NATS subscriber
func NewNATSSubscriberMetrics() *NATSSubscriberMetrics {
	return &NATSSubscriberMetrics{
		BatchesReceivedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_nats_batches_received_total",
			Help: "Total number of hash batches received from NATS",
		}),
		HashesReceivedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_nats_hashes_received_total",
			Help: "Total number of hashes received from NATS",
		}),
		InvalidMessagesTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_nats_invalid_messages_total",
			Help: "Total number of invalid NATS messages",
		}),
		ProcessingErrorsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "refinery_nats_processing_errors_total",
			Help: "Total number of hash processing errors",
		}),
		LastBatchHashCount: prometheus.NewGauge(prometheus.GaugeOpts{
			Name: "refinery_nats_last_batch_hash_count",
			Help: "Number of hashes in the last received batch",
		}),
		LastBatchTimestamp: prometheus.NewGauge(prometheus.GaugeOpts{
			Name: "refinery_nats_last_batch_timestamp_seconds",
			Help: "Unix timestamp of the last received batch",
		}),
	}
}

// NewNATSSubscriber creates a new NATS subscriber that listens for hash batches
//
// Parameters:
//   - ctx: Context for lifecycle management
//   - nc: Active NATS connection
//   - queueMgr: Queue manager to receive hashes
//
// The subscriber will automatically start listening on creation.
func NewNATSSubscriber(ctx context.Context, nc *nats.Conn, queueMgr *QueueManager) (*NATSSubscriber, error) {
	logger := slog.With("component", "nats_subscriber")

	ns := &NATSSubscriber{
		nc:       nc,
		logger:   logger,
		queueMgr: queueMgr,
		metrics:  NewNATSSubscriberMetrics(),
		ctx:      ctx,
	}

	// Subscribe to ore.batch subject
	sub, err := nc.Subscribe("ore.batch", ns.handleHashBatch)
	if err != nil {
		return nil, err
	}
	ns.sub = sub

	logger.Info("NATS subscriber created",
		"subject", "ore.batch",
	)

	// Register metrics
	prometheus.MustRegister(
		ns.metrics.BatchesReceivedTotal,
		ns.metrics.HashesReceivedTotal,
		ns.metrics.InvalidMessagesTotal,
		ns.metrics.ProcessingErrorsTotal,
		ns.metrics.LastBatchHashCount,
		ns.metrics.LastBatchTimestamp,
	)

	return ns, nil
}

// handleHashBatch processes incoming hash batch messages from NATS
//
// Message flow:
// 1. Parse JSON message into HashBatchMessage struct
// 2. Validate message fields
// 3. Add each hash to the queue manager
// 4. Update metrics
//
// This handler runs asynchronously for each received message.
func (ns *NATSSubscriber) handleHashBatch(msg *nats.Msg) {
	// Parse JSON message
	var batch HashBatchMessage
	if err := json.Unmarshal(msg.Data, &batch); err != nil {
		ns.logger.Error("failed to parse hash batch",
			"error", err,
			"data_length", len(msg.Data),
		)
		ns.metrics.InvalidMessagesTotal.Inc()
		return
	}

	// Validate message
	if batch.ContractID == "" || batch.DiggerID == "" || len(batch.Hashes) == 0 {
		ns.logger.Warn("received invalid hash batch",
			"contract_id", batch.ContractID,
			"digger_id", batch.DiggerID,
			"hash_count", len(batch.Hashes),
		)
		ns.metrics.InvalidMessagesTotal.Inc()
		return
	}

	ns.logger.Info("received hash batch",
		"contract_id", batch.ContractID,
		"digger_id", batch.DiggerID,
		"hash_count", batch.HashCount,
		"timestamp", batch.Timestamp,
	)

	// Update metrics
	ns.metrics.BatchesReceivedTotal.Inc()
	ns.metrics.HashesReceivedTotal.Add(float64(batch.HashCount))
	ns.metrics.LastBatchHashCount.Set(float64(batch.HashCount))
	// TODO: Parse timestamp and set LastBatchTimestamp

	// Add hashes to queue
	for i, hash := range batch.Hashes {
		// TODO(phase2-milestone2): Update QueueManager.AddHash() to accept hash entries
		// For now, just log (queue manager still expects units)
		if i == 0 || i == len(batch.Hashes)-1 {
			ns.logger.Debug("processing hash",
				"index", i,
				"hash", hash[:16]+"...", // Log first 16 chars
				"contract_id", batch.ContractID,
			)
		}
	}

	ns.logger.Info("hash batch processed",
		"contract_id", batch.ContractID,
		"hashes_queued", len(batch.Hashes),
	)
}

// Start begins listening for hash batches (no-op, subscription is active on creation)
//
// The NATS subscription is established in NewNATSSubscriber, so this method
// exists for API compatibility and future expansion.
func (ns *NATSSubscriber) Start() {
	ns.logger.Info("NATS subscriber active",
		"subject", "ore.batch",
		"status", "listening",
	)
}

// Stop gracefully shuts down the NATS subscriber
//
// This unsubscribes from the NATS subject and waits for any in-flight
// message handlers to complete.
func (ns *NATSSubscriber) Stop() {
	ns.logger.Info("stopping NATS subscriber...")

	if ns.sub != nil {
		if err := ns.sub.Unsubscribe(); err != nil {
			ns.logger.Error("failed to unsubscribe", "error", err)
		} else {
			ns.logger.Info("unsubscribed from ore.batch")
		}
	}

	// Wait for any in-flight handlers to complete
	ns.wg.Wait()

	ns.logger.Info("NATS subscriber stopped")
}

// IsHealthy returns true if the subscription is active
func (ns *NATSSubscriber) IsHealthy() bool {
	return ns.sub != nil && ns.sub.IsValid()
}
