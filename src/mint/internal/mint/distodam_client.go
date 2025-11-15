// Package mint provides NATS client for publishing MintEvents to DistoDam.
package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

const (
	// MintBatchesTopic is the NATS subject for broadcasting MintEvents.
	// All DistoDam instances subscribe to this topic.
	MintBatchesTopic = "mint.batches"

	// DefaultRetries is the number of publish retry attempts.
	DefaultRetries = 3

	// DefaultBackoffBase is the base duration for exponential backoff.
	DefaultBackoffBase = 100 * time.Millisecond
)

// distoDamClient implements DistoDamClient for NATS publishing.
//
// Architecture:
//   - Publishes MintEvent JSON to mint.batches topic
//   - Retries on transient failures with exponential backoff
//   - Monitors connection health
//   - Graceful degradation on persistent failures
//
// NATS Topic Structure:
//
//	mint.batches - Broadcast to all DistoDams
//	  └─ MintEvent JSON payload
type distoDamClient struct {
	natsURL     string        // NATS server URL
	conn        *nats.Conn    // NATS connection
	logger      *slog.Logger  // Structured logging
	retries     int           // Number of retry attempts
	backoffBase time.Duration // Base backoff duration

	// Prometheus metrics
	publishTotal    prometheus.Counter
	publishFailures prometheus.Counter
	publishLatency  prometheus.Histogram
	retryAttempts   prometheus.Counter
}

// NewDistoDamClient creates a new DistoDamClient instance.
//
// Parameters:
//   - natsURL: NATS server URL (e.g., "nats://nats:4222")
//   - logger: Structured logger
//
// The client is created disconnected. Call Connect() to establish connection.
func NewDistoDamClient(natsURL string, logger *slog.Logger) DistoDamClient {
	client := &distoDamClient{
		natsURL:     natsURL,
		logger:      logger,
		retries:     DefaultRetries,
		backoffBase: DefaultBackoffBase,
	}

	// Initialize Prometheus metrics
	client.publishTotal = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_nats_publish_total",
		Help: "Total number of NATS publish attempts",
	})

	client.publishFailures = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_nats_publish_failures_total",
		Help: "Total number of failed NATS publish attempts (after retries)",
	})

	client.publishLatency = promauto.NewHistogram(prometheus.HistogramOpts{
		Name:    "mint_nats_publish_seconds",
		Help:    "Time spent publishing to NATS (including retries)",
		Buckets: prometheus.DefBuckets,
	})

	client.retryAttempts = promauto.NewCounter(prometheus.CounterOpts{
		Name: "mint_nats_retry_attempts_total",
		Help: "Total number of NATS publish retry attempts",
	})

	return client
}

// Connect establishes connection to NATS server.
//
// Connection options:
//   - Name: "mint-service" (for server identification)
//   - MaxReconnects: -1 (infinite reconnection attempts)
//   - ReconnectWait: 2s (delay between reconnection attempts)
//   - DisconnectErrHandler: logs disconnection events
//   - ReconnectHandler: logs successful reconnection
//
// Returns error if initial connection fails.
func (c *distoDamClient) Connect() error {
	opts := []nats.Option{
		nats.Name("mint-service"),
		nats.MaxReconnects(-1), // Infinite reconnects
		nats.ReconnectWait(2 * time.Second),
		nats.DisconnectErrHandler(func(nc *nats.Conn, err error) {
			if err != nil {
				c.logger.Error("NATS disconnected",
					"error", err,
					"url", c.natsURL,
				)
			}
		}),
		nats.ReconnectHandler(func(nc *nats.Conn) {
			c.logger.Info("NATS reconnected",
				"url", nc.ConnectedUrl(),
			)
		}),
	}

	conn, err := nats.Connect(c.natsURL, opts...)
	if err != nil {
		c.logger.Error("failed to connect to NATS",
			"error", err,
			"url", c.natsURL,
		)
		return fmt.Errorf("failed to connect to NATS: %w", err)
	}

	c.conn = conn

	c.logger.Info("connected to NATS",
		"url", c.natsURL,
		"server_name", conn.ConnectedServerName(),
	)

	return nil
}

// Publish sends a MintEvent to the mint.batches topic.
//
// Flow:
//  1. Marshal MintEvent to JSON
//  2. Attempt publish with retries (exponential backoff)
//  3. Update metrics (success/failure)
//  4. Log result
//
// Retry logic:
//   - Attempt 1: immediate
//   - Attempt 2: wait 100ms (backoffBase * 2^0)
//   - Attempt 3: wait 200ms (backoffBase * 2^1)
//   - Attempt 4: wait 400ms (backoffBase * 2^2)
//
// Returns error if all retry attempts fail.
func (c *distoDamClient) Publish(ctx context.Context, event *MintEvent) error {
	startTime := time.Now()

	// Marshal event to JSON
	data, err := json.Marshal(event)
	if err != nil {
		c.publishFailures.Inc()
		return fmt.Errorf("failed to marshal MintEvent: %w", err)
	}

	// Publish with retry logic
	var lastErr error
	for attempt := 0; attempt <= c.retries; attempt++ {
		if attempt > 0 {
			// Exponential backoff: 100ms, 200ms, 400ms
			backoff := c.backoffBase * time.Duration(1<<uint(attempt-1))
			c.logger.Debug("retrying NATS publish",
				"attempt", attempt,
				"backoff_ms", backoff.Milliseconds(),
				"batch_id", event.BatchID,
			)
			c.retryAttempts.Inc()

			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(backoff):
				// Continue to retry
			}
		}

		// Attempt publish
		c.publishTotal.Inc()
		err = c.conn.Publish(MintBatchesTopic, data)
		if err == nil {
			// Success!
			c.publishLatency.Observe(time.Since(startTime).Seconds())
			c.logger.Info("published MintEvent to NATS",
				"batch_id", event.BatchID,
				"batch_hash", event.BatchHash,
				"topic", MintBatchesTopic,
				"ingots", event.IngotsProcessed,
				"total_robo", event.TotalRoboTorq,
				"attempts", attempt+1,
				"duration_ms", time.Since(startTime).Milliseconds(),
			)
			return nil
		}

		lastErr = err
		c.logger.Warn("NATS publish attempt failed",
			"attempt", attempt+1,
			"max_attempts", c.retries+1,
			"error", err,
			"batch_id", event.BatchID,
		)
	}

	// All retries exhausted
	c.publishFailures.Inc()
	c.publishLatency.Observe(time.Since(startTime).Seconds())

	c.logger.Error("NATS publish failed after all retries",
		"batch_id", event.BatchID,
		"attempts", c.retries+1,
		"error", lastErr,
		"duration_ms", time.Since(startTime).Milliseconds(),
	)

	return fmt.Errorf("failed to publish after %d attempts: %w", c.retries+1, lastErr)
}

// Close gracefully shuts down the NATS connection.
//
// Flow:
//  1. Drain outstanding messages (with timeout)
//  2. Close connection
//  3. Log shutdown
func (c *distoDamClient) Close() error {
	if c.conn == nil {
		return nil
	}

	// Drain with 5-second timeout
	// This ensures all published messages are flushed before closing
	drainTimeout := 5 * time.Second
	done := make(chan error, 1)

	go func() {
		done <- c.conn.Drain()
	}()

	select {
	case err := <-done:
		if err != nil {
			c.logger.Warn("error draining NATS connection",
				"error", err,
			)
		}
	case <-time.After(drainTimeout):
		c.logger.Warn("NATS drain timed out",
			"timeout", drainTimeout,
		)
	}

	// Close connection
	c.conn.Close()

	c.logger.Info("NATS connection closed",
		"url", c.natsURL,
	)

	return nil
}

// IsConnected returns true if connected to NATS server.
func (c *distoDamClient) IsConnected() bool {
	return c.conn != nil && c.conn.IsConnected()
}

// GetConnection returns the underlying NATS connection.
// Used by IngotReceiver to subscribe to mint.ingots topic.
func (c *distoDamClient) GetConnection() *nats.Conn {
	return c.conn
}
