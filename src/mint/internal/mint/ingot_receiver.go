// Package mint provides the IngotReceiver component.
// IngotReceiver handles HTTP ingot reception from Refinery.
package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/prometheus/client_golang/prometheus"
)

// ─────────────────────────────────────────────────────────────
// IngotReceiver Implementation
// ─────────────────────────────────────────────────────────────

// ingotReceiver implements the IngotReceiver interface.
// It handles HTTP POST /mint-tokentorq requests from Refinery.
type ingotReceiver struct {
	buffer  IngotBuffer
	server  *http.Server
	metrics *ingotReceiverMetrics
	logger  *slog.Logger
}

// ingotReceiverMetrics holds Prometheus metrics for IngotReceiver.
type ingotReceiverMetrics struct {
	ingotsReceived    prometheus.Counter
	ingotsRejected    prometheus.Counter
	validationErrors  prometheus.Counter
	backpressureCount prometheus.Counter
}

// NewIngotReceiver creates a new IngotReceiver instance.
func NewIngotReceiver(buffer IngotBuffer, port string, logger *slog.Logger) IngotReceiver {
	metrics := &ingotReceiverMetrics{
		ingotsReceived: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_ingots_received_total",
			Help: "Total number of ingots received from Refinery",
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
	}

	// Register metrics
	prometheus.MustRegister(
		metrics.ingotsReceived,
		metrics.ingotsRejected,
		metrics.validationErrors,
		metrics.backpressureCount,
	)

	receiver := &ingotReceiver{
		buffer:  buffer,
		metrics: metrics,
		logger:  logger,
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
func (r *ingotReceiver) ReceiveIngot(ingot *TokenTorqIngot) error {
	// Validate ingot
	if err := r.validateIngot(ingot); err != nil {
		r.metrics.validationErrors.Inc()
		r.metrics.ingotsRejected.Inc()
		r.logger.Error("ingot validation failed",
			"error", err,
			"joule", ingot.JouleTorq,
			"robo", ingot.RoboTorq,
			"price", ingot.Price,
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
		"joule", ingot.JouleTorq,
		"robo", ingot.RoboTorq,
		"price", ingot.Price,
		"contract", ingot.ContractID,
		"digger", ingot.DiggerID,
		"buffer_len", r.buffer.Len(),
	)

	return nil
}

// Start begins the HTTP server.
func (r *ingotReceiver) Start(ctx context.Context) error {
	r.logger.Info("starting ingot receiver",
		"addr", r.server.Addr,
	)

	// Start server in goroutine
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

// Shutdown gracefully stops the HTTP server.
func (r *ingotReceiver) Shutdown(ctx context.Context) error {
	r.logger.Info("shutting down ingot receiver")
	return r.server.Shutdown(ctx)
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
//   - JouleTorq must be exactly 3600.0
//   - RoboTorq must be >= 0
//   - Price must be > 0
//   - ContractID, DiggerID, Hash must not be empty
func (r *ingotReceiver) validateIngot(ingot *TokenTorqIngot) error {
	// Validate JouleTorq (must be exactly 3600)
	if ingot.JouleTorq != 3600.0 {
		return fmt.Errorf("invalid JouleTorq: got %.2f, expected 3600.0", ingot.JouleTorq)
	}

	// Validate RoboTorq (must be non-negative)
	if ingot.RoboTorq < 0 {
		return fmt.Errorf("invalid RoboTorq: must be >= 0, got %.6f", ingot.RoboTorq)
	}

	// Validate Price (must be positive)
	if ingot.Price <= 0 {
		return fmt.Errorf("invalid Price: must be > 0, got %.2f", ingot.Price)
	}

	// Validate metadata fields (must not be empty)
	if ingot.ContractID == "" {
		return fmt.Errorf("contract ID cannot be empty")
	}

	if ingot.DiggerID == "" {
		return fmt.Errorf("digger ID cannot be empty")
	}

	if ingot.Hash == "" {
		return fmt.Errorf("hash cannot be empty")
	}

	// Validate timestamp (must not be zero)
	if ingot.Timestamp.IsZero() {
		return fmt.Errorf("timestamp cannot be zero")
	}

	return nil
}
