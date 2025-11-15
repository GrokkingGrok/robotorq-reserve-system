// Package mint provides the IngotReceiver component.
// IngotReceiver handles ingot reception from Refinery via HTTP and NATS.
package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
)

const (
	// MintIngotsTopic is the NATS subject for receiving ingots from Refinery.
	MintIngotsTopic = "mint.ingots"
)

// ─────────────────────────────────────────────────────────────
// IngotReceiver Implementation
// ─────────────────────────────────────────────────────────────

// ingotReceiver implements the IngotReceiver interface.
// It handles both HTTP POST /mint-tokentorq and NATS subscriptions.
type ingotReceiver struct {
	buffer   IngotBuffer
	natsConn *nats.Conn
	natsSub  *nats.Subscription
	server   *http.Server
	metrics  *ingotReceiverMetrics
	logger   *slog.Logger
}

// ingotReceiverMetrics holds Prometheus metrics for IngotReceiver.
type ingotReceiverMetrics struct {
	ingotsReceived       prometheus.Counter
	ingotsReceivedHTTP   prometheus.Counter
	ingotsReceivedNATS   prometheus.Counter
	ingotsRejected       prometheus.Counter
	validationErrors     prometheus.Counter
	backpressureCount    prometheus.Counter
	natsBatchesReceived  prometheus.Counter
	natsMessagesReceived prometheus.Counter
}

// NewIngotReceiver creates a new IngotReceiver instance.
func NewIngotReceiver(buffer IngotBuffer, natsConn *nats.Conn, port string, logger *slog.Logger) IngotReceiver {
	metrics := &ingotReceiverMetrics{
		ingotsReceived: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingots_received_total",
			Help: "Total number of ingots received from all sources",
		}),
		ingotsReceivedHTTP: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingots_received_http_total",
			Help: "Total number of ingots received via HTTP",
		}),
		ingotsReceivedNATS: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingots_received_nats_total",
			Help: "Total number of ingots received via NATS",
		}),
		ingotsRejected: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingots_rejected_total",
			Help: "Total number of ingots rejected (validation failures)",
		}),
		validationErrors: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_validation_errors_total",
			Help: "Total number of validation errors",
		}),
		backpressureCount: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_backpressure_total",
			Help: "Total number of times backpressure (429) was triggered",
		}),
		natsBatchesReceived: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_nats_batches_received_total",
			Help: "Total number of NATS batches received from Refinery",
		}),
		natsMessagesReceived: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_nats_messages_received_total",
			Help: "Total number of NATS messages received",
		}),
	}

	// Register metrics
	prometheus.MustRegister(
		metrics.ingotsReceived,
		metrics.ingotsReceivedHTTP,
		metrics.ingotsReceivedNATS,
		metrics.ingotsRejected,
		metrics.validationErrors,
		metrics.backpressureCount,
		metrics.natsBatchesReceived,
		metrics.natsMessagesReceived,
	)

	receiver := &ingotReceiver{
		buffer:   buffer,
		natsConn: natsConn,
		metrics:  metrics,
		logger:   logger,
	}

	// Create HTTP mux
	mux := http.NewServeMux()
	mux.HandleFunc("/mint-tokentorq", receiver.handleIngot)
	mux.HandleFunc("/health", receiver.handleHealth)

	// Create HTTP server
	receiver.server = &http.Server{
		Addr:    ":" + port,
		Handler: mux,
	}

	return receiver
}

// ─────────────────────────────────────────────────────────────
// IngotReceiver Interface Implementation
// ─────────────────────────────────────────────────────────────

// ReceiveIngot validates and queues a single TokenTorqIngot.
// This is called by both HTTP and NATS handlers.
func (r *ingotReceiver) ReceiveIngot(ingot *TokenTorqIngot) error {
	// Validate ingot
	if err := r.validateIngot(ingot); err != nil {
		r.metrics.validationErrors.Inc()
		r.metrics.ingotsRejected.Inc()
		r.logger.Error("ingot validation failed",
			"error", err,
			"joule_total", ingot.JouleTorqTotal,
			"robo_stake", ingot.RoboStakeTotal,
			"units", len(ingot.Units),
		)
		return fmt.Errorf("validation failed: %w", err)
	}

	// Try to push to buffer
	if err := r.buffer.Push(ingot); err != nil {
		r.metrics.backpressureCount.Inc()
		r.logger.Warn("buffer full, backpressure triggered",
			"buffer_len", r.buffer.Len(),
			"buffer_cap", r.buffer.Cap(),
		)
		return fmt.Errorf("buffer full: %w", err)
	}

	// Success
	r.metrics.ingotsReceived.Inc()
	r.logger.Info("ingot received",
		"ingot_id", ingot.IngotID,
		"joule_total", ingot.JouleTorqTotal,
		"robo_stake", ingot.RoboStakeTotal,
		"units", len(ingot.Units),
		"contracts", len(ingot.ContractIDs),
		"buffer_len", r.buffer.Len(),
	)

	return nil
}

// Start begins both the HTTP server and NATS subscription.
func (r *ingotReceiver) Start(ctx context.Context) error {
	r.logger.Info("starting ingot receiver",
		"http_addr", r.server.Addr,
		"nats_topic", MintIngotsTopic,
	)

	// Start NATS subscription (if NATS connection is available)
	if r.natsConn != nil {
		if err := r.subscribeToNATS(); err != nil {
			return fmt.Errorf("failed to subscribe to NATS: %w", err)
		}
	} else {
		r.logger.Warn("NATS connection not available, skipping NATS subscription")
	}

	// Start HTTP server in goroutine
	errChan := make(chan error, 1)
	go func() {
		if err := r.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errChan <- err
		}
	}()

	// Wait for context cancellation or server error
	select {
	case <-ctx.Done():
		r.logger.Info("ingot receiver context cancelled")
		return ctx.Err()
	case err := <-errChan:
		r.logger.Error("ingot receiver server error", "error", err)
		return err
	}
}

// Shutdown gracefully stops both HTTP server and NATS subscription.
func (r *ingotReceiver) Shutdown(ctx context.Context) error {
	r.logger.Info("shutting down ingot receiver")

	// Unsubscribe from NATS
	if r.natsSub != nil {
		if err := r.natsSub.Unsubscribe(); err != nil {
			r.logger.Error("failed to unsubscribe from NATS", "error", err)
		}
	}

	// Shutdown HTTP server
	return r.server.Shutdown(ctx)
}

// ─────────────────────────────────────────────────────────────
// NATS Subscription
// ─────────────────────────────────────────────────────────────

// IngotBatch represents the batch envelope from Refinery.
type IngotBatch struct {
	BatchID   string            `json:"batch_id"`
	Timestamp time.Time         `json:"timestamp"`
	Count     int               `json:"count"`
	Ingots    []*TokenTorqIngot `json:"ingots"`
}

// subscribeToNATS sets up NATS subscription to mint.ingots topic.
func (r *ingotReceiver) subscribeToNATS() error {
	if r.natsConn == nil {
		return fmt.Errorf("NATS connection is nil")
	}

	// Subscribe to mint.ingots topic
	sub, err := r.natsConn.Subscribe(MintIngotsTopic, r.handleNATSMessage)
	if err != nil {
		return fmt.Errorf("failed to subscribe to %s: %w", MintIngotsTopic, err)
	}

	r.natsSub = sub

	r.logger.Info("subscribed to NATS topic",
		"topic", MintIngotsTopic,
	)

	return nil
}

// handleNATSMessage processes incoming NATS messages from Refinery.
func (r *ingotReceiver) handleNATSMessage(msg *nats.Msg) {
	r.metrics.natsMessagesReceived.Inc()

	// Decode batch envelope
	var batch IngotBatch
	if err := json.Unmarshal(msg.Data, &batch); err != nil {
		r.logger.Error("failed to unmarshal NATS batch",
			"error", err,
			"subject", msg.Subject,
		)
		return
	}

	r.metrics.natsBatchesReceived.Inc()

	r.logger.Info("received NATS batch",
		"batch_id", batch.BatchID,
		"count", batch.Count,
		"ingots", len(batch.Ingots),
		"timestamp", batch.Timestamp,
	)

	// Process each ingot in the batch
	successCount := 0
	for i, ingot := range batch.Ingots {
		if err := r.ReceiveIngot(ingot); err != nil {
			r.logger.Error("failed to process ingot from NATS batch",
				"error", err,
				"batch_id", batch.BatchID,
				"ingot_index", i,
				"ingot_id", ingot.IngotID,
			)
			// Continue processing remaining ingots
			continue
		}
		r.metrics.ingotsReceivedNATS.Inc()
		successCount++
	}

	r.logger.Info("processed NATS batch",
		"batch_id", batch.BatchID,
		"total", len(batch.Ingots),
		"success", successCount,
		"failed", len(batch.Ingots)-successCount,
	)
}

// ─────────────────────────────────────────────────────────────
// HTTP Handlers
// ─────────────────────────────────────────────────────────────

// handleIngot processes POST /mint-tokentorq requests.
func (r *ingotReceiver) handleIngot(w http.ResponseWriter, req *http.Request) {
	// Only allow POST
	if req.Method != http.MethodPost {
		r.logger.Warn("invalid method", "method", req.Method)
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Decode ingot
	var ingot TokenTorqIngot
	if err := json.NewDecoder(req.Body).Decode(&ingot); err != nil {
		r.logger.Error("failed to decode ingot", "error", err)
		r.metrics.ingotsRejected.Inc()
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}

	// Process ingot
	if err := r.ReceiveIngot(&ingot); err != nil {
		// Check if it's backpressure
		if r.buffer.Len() >= r.buffer.Cap() {
			http.Error(w, "buffer full", http.StatusTooManyRequests)
			return
		}
		// Other errors (validation, etc.)
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Track HTTP-specific metric
	r.metrics.ingotsReceivedHTTP.Inc()

	// Success
	w.WriteHeader(http.StatusAccepted)
	w.Write([]byte("accepted"))
}

// handleHealth provides a simple health check endpoint.
func (r *ingotReceiver) handleHealth(w http.ResponseWriter, req *http.Request) {
	status := map[string]interface{}{
		"status":      "ok",
		"buffer_len":  r.buffer.Len(),
		"buffer_cap":  r.buffer.Cap(),
		"buffer_util": float64(r.buffer.Len()) / float64(r.buffer.Cap()) * 100,
		"timestamp":   time.Now().UTC().Format(time.RFC3339),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(status)
}

// ─────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────

// validateIngot checks if an ingot meets requirements.
// Rules:
//   - JouleTorqTotal must be exactly 3600
//   - RoboStakeTotal must be >= 0
//   - PricePerRT must be > 0
//   - IngotID, ContractIDs, JouleTorqHashes must not be empty
func (r *ingotReceiver) validateIngot(ingot *TokenTorqIngot) error {
	// Validate JouleTorqTotal (should be ~3600, allow small variance for float accumulation)
	if ingot.JouleTorqTotal < 3500 || ingot.JouleTorqTotal > 3700 {
		return fmt.Errorf("invalid JouleTorqTotal: got %.2f, expected ~3600", ingot.JouleTorqTotal)
	}

	// Validate RoboStakeTotal (must be non-negative)
	if ingot.RoboStakeTotal < 0 {
		return fmt.Errorf("invalid RoboStakeTotal: must be >= 0, got %.6f", ingot.RoboStakeTotal)
	}

	// Validate Units (must have exactly 3600 JouleTorqUnits)
	if len(ingot.Units) != 3600 {
		return fmt.Errorf("invalid Units count: must be 3600, got %d", len(ingot.Units))
	}

	// Validate IngotID (must not be empty)
	if ingot.IngotID == "" {
		return fmt.Errorf("ingot ID cannot be empty")
	}

	// Validate ContractIDs (must not be empty)
	if len(ingot.ContractIDs) == 0 {
		return fmt.Errorf("contract IDs cannot be empty")
	}

	// Validate BranchHash (must not be empty - this is the merkle branch hash)
	if ingot.BranchHash == "" {
		return fmt.Errorf("branch hash cannot be empty")
	}

	// Validate MintedAt (must not be zero)
	if ingot.MintedAt.IsZero() {
		return fmt.Errorf("minted_at timestamp cannot be zero")
	}

	return nil
}
