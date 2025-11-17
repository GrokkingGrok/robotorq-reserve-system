// internal/refinery/mint_client.go
// NATS client for publishing TokenTorq Ingots to Mint service

package refinery

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"b2b/refinery/internal/config"
	"b2b/refinery/internal/models"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
)

const (
	// MintIngotsTopic is the NATS topic for sending Phase 2 ingots to Mint
	// Using separate topic to avoid conflict with old Phase 1-4 TokenTorqIngot receiver
	MintIngotsTopic = "mint.phase2.ingots"
)

var (
	// mintPublishTotal tracks successful publishes to Mint
	mintPublishTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_mint_publish_total",
		Help: "Total successful publishes to Mint",
	})

	// mintPublishFailures tracks failed publish attempts
	mintPublishFailures = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_mint_publish_failures_total",
		Help: "Total failed publish attempts to Mint",
	})

	// mintPublishDuration tracks publish latency
	mintPublishDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_mint_publish_duration_seconds",
		Help:    "Time taken to publish to Mint",
		Buckets: prometheus.DefBuckets,
	})

	// mintRetryAttempts tracks retry attempts per batch
	mintRetryAttempts = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_mint_retry_attempts",
		Help:    "Number of retry attempts per batch",
		Buckets: []float64{0, 1, 2, 3},
	})
)

func init() {
	prometheus.MustRegister(mintPublishTotal)
	prometheus.MustRegister(mintPublishFailures)
	prometheus.MustRegister(mintPublishDuration)
	prometheus.MustRegister(mintRetryAttempts)
}

// MintClient handles NATS communication with Mint service
type MintClient struct {
	conn   *nats.Conn
	config *config.Config
	ctx    context.Context
}

// NewMintClient creates a new Mint client with NATS connection
func NewMintClient(ctx context.Context, cfg *config.Config) (*MintClient, error) {
	slog.Info("connecting to NATS",
		"url", cfg.NatsURL,
	)

	nc, err := nats.Connect(cfg.NatsURL)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to NATS: %w", err)
	}

	slog.Info("connected to NATS successfully",
		"url", cfg.NatsURL,
		"status", nc.Status(),
	)

	return &MintClient{
		conn:   nc,
		config: cfg,
		ctx:    ctx,
	}, nil
}

// PublishBatch sends a batch of ingots to Mint with retry logic
func (mc *MintClient) PublishBatch(ingots []*models.TokenTorqIngot) error {
	if len(ingots) == 0 {
		return nil
	}

	timer := prometheus.NewTimer(mintPublishDuration)
	defer timer.ObserveDuration()

	// Wrap ingots in a batch envelope
	batch := map[string]interface{}{
		"batch_id":  fmt.Sprintf("batch-%d", time.Now().Unix()),
		"timestamp": time.Now().UTC(),
		"count":     len(ingots),
		"ingots":    ingots,
	}

	slog.Info("publishing batch to mint",
		"topic", MintIngotsTopic,
		"batch_size", len(ingots),
	)

	// Publish with retry logic
	err := mc.publishWithRetry(MintIngotsTopic, batch)
	if err != nil {
		mintPublishFailures.Inc()
		return err
	}

	mintPublishTotal.Inc()
	slog.Info("batch published successfully",
		"topic", MintIngotsTopic,
		"batch_size", len(ingots),
	)

	return nil
}

// PublishPhase2Batch sends a batch of Phase2 ingots to Mint with retry logic
func (mc *MintClient) PublishPhase2Batch(ingots []*Phase2Ingot) error {
	if len(ingots) == 0 {
		return nil
	}

	timer := prometheus.NewTimer(mintPublishDuration)
	defer timer.ObserveDuration()

	// Wrap Phase2 ingots in a batch envelope
	batch := map[string]interface{}{
		"batch_id":  fmt.Sprintf("phase2-batch-%d", time.Now().Unix()),
		"timestamp": time.Now().UTC(),
		"count":     len(ingots),
		"ingots":    ingots,
	}

	slog.Info("publishing Phase2 batch to mint",
		"topic", MintIngotsTopic,
		"batch_size", len(ingots),
	)

	// Publish with retry logic
	err := mc.publishWithRetry(MintIngotsTopic, batch)
	if err != nil {
		mintPublishFailures.Inc()
		return err
	}

	mintPublishTotal.Inc()
	slog.Info("Phase2 batch published successfully",
		"topic", MintIngotsTopic,
		"batch_size", len(ingots),
	)

	return nil
}

// publishWithRetry implements exponential backoff retry logic
func (mc *MintClient) publishWithRetry(subject string, data interface{}) error {
	maxRetries := mc.config.MintMaxRetries
	baseDelay := mc.config.MintBaseDelay

	var lastErr error
	for attempt := 0; attempt <= maxRetries; attempt++ {
		// Marshal data to JSON
		jsonData, err := json.Marshal(data)
		if err != nil {
			return fmt.Errorf("failed to marshal batch: %w", err)
		}

		// Attempt to publish
		err = mc.conn.Publish(subject, jsonData)
		if err == nil {
			// Success
			if attempt > 0 {
				mintRetryAttempts.Observe(float64(attempt))
				slog.Info("publish succeeded after retry",
					"attempt", attempt+1,
					"subject", subject,
				)
			}
			return nil
		}

		lastErr = err

		// If this was the last attempt, give up
		if attempt == maxRetries {
			mintRetryAttempts.Observe(float64(maxRetries))
			slog.Error("publish failed after all retries",
				"attempts", maxRetries+1,
				"subject", subject,
				"error", err,
			)
			break
		}

		// Calculate exponential backoff delay
		delay := baseDelay * time.Duration(1<<uint(attempt)) // 1s, 2s, 4s, 8s...

		slog.Warn("publish failed, retrying",
			"attempt", attempt+1,
			"max_retries", maxRetries+1,
			"delay", delay,
			"subject", subject,
			"error", err,
		)

		// Wait before retry (with context cancellation support)
		select {
		case <-time.After(delay):
			// Continue to retry
		case <-mc.ctx.Done():
			return fmt.Errorf("publish cancelled during retry: %w", mc.ctx.Err())
		}
	}

	return fmt.Errorf("publish failed after %d attempts: %w", maxRetries+1, lastErr)
}

// IsConnected checks if NATS connection is active
func (mc *MintClient) IsConnected() bool {
	return mc.conn != nil && mc.conn.IsConnected()
}

// Connection returns the underlying NATS connection for sharing with other components
// This allows the NATSSubscriber to use the same connection as MintClient
func (mc *MintClient) Connection() *nats.Conn {
	return mc.conn
}

// GetStatus returns the current NATS connection status
func (mc *MintClient) GetStatus() string {
	if mc.conn == nil {
		return "disconnected"
	}
	return mc.conn.Status().String()
}

// Close gracefully closes the NATS connection
func (mc *MintClient) Close() {
	if mc.conn != nil {
		slog.Info("closing mint client NATS connection")
		mc.conn.Close()
	}
}
