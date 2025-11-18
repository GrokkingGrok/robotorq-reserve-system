// event_publisher.go - NATS event publisher with retry logic
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"github.com/nats-io/nats.go"
)

// EventPublisher publishes DistoDam events to NATS topics
// Phase 6 MVP: Publishes contract funding and UBD distribution events
type EventPublisher struct {
	nc      *nats.Conn
	logger  *slog.Logger
	metrics *VaultMetrics

	// Retry configuration
	maxRetries     int
	initialBackoff time.Duration
}

// NewEventPublisher creates a new EventPublisher
func NewEventPublisher(nc *nats.Conn, logger *slog.Logger, metrics *VaultMetrics) *EventPublisher {
	return &EventPublisher{
		nc:             nc,
		logger:         logger,
		metrics:        metrics,
		maxRetries:     3,                      // 3 attempts total (initial + 2 retries)
		initialBackoff: 100 * time.Millisecond, // Start with 100ms
	}
}

// ContractFundedEvent represents a contract funding event
type ContractFundedEvent struct {
	EventID    string    `json:"event_id"`          // Unique event ID
	ContractID string    `json:"contract_id"`       // Contract that was funded
	AmountRT   float64   `json:"amount_rt"`         // Amount funded in RT
	Source     string    `json:"source"`            // "stake_vault" or "disto_vault_loan"
	LoanID     string    `json:"loan_id,omitempty"` // Loan ID if funded via loan
	Timestamp  time.Time `json:"timestamp"`         // When funding occurred
	VaultRatio float64   `json:"vault_ratio"`       // StakeVault / DistoVault ratio
}

// UBDFundedEvent represents a UBD distribution event
type UBDFundedEvent struct {
	EventID      string    `json:"event_id"`      // Unique event ID
	AmountRT     float64   `json:"amount_rt"`     // Amount distributed in RT
	RecipientID  string    `json:"recipient_id"`  // UBD recipient
	Timestamp    time.Time `json:"timestamp"`     // When distribution occurred
	DistoBalance float64   `json:"disto_balance"` // DistoVault balance after distribution
}

// PublishContractFunded publishes a contract funding event to NATS
func (ep *EventPublisher) PublishContractFunded(ctx context.Context, event *ContractFundedEvent) error {
	topic := "contracts.funded"

	ep.logger.Info("publishing contract funded event",
		"event_id", event.EventID,
		"contract_id", event.ContractID,
		"amount_rt", event.AmountRT,
		"source", event.Source,
		"topic", topic)

	return ep.publishWithRetry(ctx, topic, event)
}

// PublishUBDFunded publishes a UBD distribution event to NATS
func (ep *EventPublisher) PublishUBDFunded(ctx context.Context, event *UBDFundedEvent) error {
	topic := "ubd.funded"

	ep.logger.Info("publishing UBD funded event",
		"event_id", event.EventID,
		"recipient_id", event.RecipientID,
		"amount_rt", event.AmountRT,
		"topic", topic)

	return ep.publishWithRetry(ctx, topic, event)
}

// publishWithRetry publishes a message with exponential backoff retry logic
func (ep *EventPublisher) publishWithRetry(ctx context.Context, topic string, event interface{}) error {
	// Serialize to JSON
	data, err := json.Marshal(event)
	if err != nil {
		ep.logger.Error("failed to serialize event",
			"topic", topic,
			"error", err)
		return fmt.Errorf("serialize event: %w", err)
	}

	// Attempt publish with retries
	var lastErr error
	backoff := ep.initialBackoff

	for attempt := 0; attempt < ep.maxRetries; attempt++ {
		// Check context cancellation before each attempt
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		// Attempt publish
		err := ep.nc.Publish(topic, data)
		if err == nil {
			// Success!
			ep.metrics.PublishSuccessTotal.Inc()

			if attempt > 0 {
				ep.logger.Info("publish succeeded after retry",
					"topic", topic,
					"attempt", attempt+1)
			}

			return nil
		}

		// Record failure
		lastErr = err
		ep.metrics.PublishFailuresTotal.Inc()

		// Don't retry on last attempt
		if attempt < ep.maxRetries-1 {
			ep.metrics.PublishRetryTotal.Inc()

			ep.logger.Warn("publish failed, retrying",
				"topic", topic,
				"attempt", attempt+1,
				"backoff_ms", backoff.Milliseconds(),
				"error", err)

			// Wait with exponential backoff
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(backoff):
			}

			// Exponential backoff: 100ms, 200ms, 400ms, 800ms, ...
			backoff *= 2
		}
	}

	// All retries exhausted
	ep.logger.Error("publish failed after all retries",
		"topic", topic,
		"attempts", ep.maxRetries,
		"error", lastErr)

	return fmt.Errorf("publish to %s after %d attempts: %w", topic, ep.maxRetries, lastErr)
}

// Flush ensures all buffered messages are sent to NATS server
func (ep *EventPublisher) Flush(ctx context.Context) error {
	// NATS Flush blocks until all messages are sent
	// We need to respect context cancellation

	done := make(chan error, 1)
	go func() {
		done <- ep.nc.Flush()
	}()

	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-done:
		if err != nil {
			ep.logger.Error("failed to flush NATS connection", "error", err)
			return fmt.Errorf("flush NATS: %w", err)
		}
		return nil
	}
}

// Close closes the EventPublisher (does NOT close NATS connection)
func (ep *EventPublisher) Close() error {
	// EventPublisher doesn't own the NATS connection, so we don't close it
	// Just log that we're shutting down
	ep.logger.Info("event publisher closing")
	return nil
}
