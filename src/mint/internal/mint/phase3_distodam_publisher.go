// Package mint provides NATS publisher for Phase3RoboTorqUnit to DistoDam.
package mint

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"b2b/mint/internal/models"

	"github.com/nats-io/nats.go"
)

const (
	// Phase3UnitsTopic is the NATS subject for broadcasting Phase3RoboTorqUnit.
	// All DistoDam instances subscribe to this topic.
	Phase3UnitsTopic = "distodam.units"

	// DefaultPublishRetries is the number of publish retry attempts.
	DefaultPublishRetries = 3

	// DefaultPublishBackoffBase is the base duration for exponential backoff.
	DefaultPublishBackoffBase = 100 * time.Millisecond
)

// Phase3DistoDamPublisher consumes Phase3RoboTorqUnit from assembler and publishes to NATS.
//
// Architecture:
//   - Consumes: Phase3RoboTorqUnit from Phase3Assembler.GetUnitChannel()
//   - Publishes: JSON payload to distodam.units topic
//   - Retries: Exponential backoff on transient failures
//   - Graceful shutdown: Drains channel, publishes remaining units
//
// NATS Topic Structure:
//
//	distodam.units - Broadcast to all DistoDams
//	  └─ Phase3RoboTorqUnit JSON payload
//
// Flow:
//  1. Wait for Phase3RoboTorqUnit on channel
//  2. Serialize to JSON
//  3. Publish to NATS with retry logic
//  4. Update metrics
//  5. Repeat until context cancelled
//  6. On shutdown: drain channel, publish all remaining units
type Phase3DistoDamPublisher struct {
	natsConn    *nats.Conn                        // NATS connection (shared)
	unitChannel <-chan *models.Phase3RoboTorqUnit // Input from assembler
	logger      *slog.Logger                      // Structured logging
	metrics     *Phase3DistoDamPublisherMetrics   // Prometheus metrics
	retries     int                               // Number of retry attempts
	backoffBase time.Duration                     // Base backoff duration
}

// NewPhase3DistoDamPublisher creates a new Phase3 DistoDam publisher.
//
// Parameters:
//   - natsConn: Shared NATS connection
//   - unitChannel: Channel from Phase3Assembler.GetUnitChannel()
//   - logger: Structured logger
//   - metrics: Prometheus metrics (optional, uses default if nil)
//
// The publisher is created ready to start. Call Start(ctx) to begin consuming.
func NewPhase3DistoDamPublisher(
	natsConn *nats.Conn,
	unitChannel <-chan *models.Phase3RoboTorqUnit,
	logger *slog.Logger,
	metrics *Phase3DistoDamPublisherMetrics,
) *Phase3DistoDamPublisher {
	if metrics == nil {
		metrics = NewPhase3DistoDamPublisherMetrics(nil) // Use default registry
	}

	return &Phase3DistoDamPublisher{
		natsConn:    natsConn,
		unitChannel: unitChannel,
		logger:      logger,
		metrics:     metrics,
		retries:     DefaultPublishRetries,
		backoffBase: DefaultPublishBackoffBase,
	}
}

// Start begins consuming Phase3RoboTorqUnit and publishing to NATS.
//
// Flow:
//  1. Wait for unit on channel
//  2. Publish to NATS (with retry)
//  3. Update metrics
//  4. Repeat until context cancelled OR channel closed
//  5. On shutdown: drain remaining units from channel
//
// Blocks until context is cancelled or channel is closed.
func (p *Phase3DistoDamPublisher) Start(ctx context.Context) {
	p.logger.Info("Phase3DistoDamPublisher started",
		"topic", Phase3UnitsTopic,
		"retries", p.retries,
	)

	for {
		select {
		case <-ctx.Done():
			p.logger.Info("Phase3DistoDamPublisher shutdown requested",
				"reason", ctx.Err())
			p.drainRemainingUnits(ctx)
			p.logger.Info("Phase3DistoDamPublisher stopped")
			return

		case unit, ok := <-p.unitChannel:
			if !ok {
				// Channel closed (assembler stopped)
				p.logger.Info("Unit channel closed, shutting down publisher")
				return
			}

			// Publish unit to NATS
			if err := p.publishUnit(ctx, unit); err != nil {
				p.logger.Error("Failed to publish Phase3RoboTorqUnit",
					"error", err,
					"unit_id", unit.UnitID,
					"merkle_root", unit.MerkleRoot,
				)
				p.metrics.PublishErrorsTotal.Inc()
				// Note: Not fatal - continue consuming next unit
			} else {
				p.metrics.UnitsPublishedTotal.Inc()
				p.logger.Info("Published Phase3RoboTorqUnit",
					"unit_id", unit.UnitID,
					"merkle_root", unit.MerkleRoot,
					"topic", Phase3UnitsTopic,
				)
			}
		}
	}
}

// publishUnit publishes a single Phase3RoboTorqUnit to NATS with retry logic.
//
// Retry logic:
//   - Attempt 1: immediate
//   - Attempt 2: wait 100ms (backoffBase * 2^0)
//   - Attempt 3: wait 200ms (backoffBase * 2^1)
//   - Attempt 4: wait 400ms (backoffBase * 2^2)
//
// Returns error if all retry attempts fail.
func (p *Phase3DistoDamPublisher) publishUnit(ctx context.Context, unit *models.Phase3RoboTorqUnit) error {
	startTime := time.Now()

	// Validate unit
	if err := unit.Validate(); err != nil {
		return fmt.Errorf("invalid Phase3RoboTorqUnit: %w", err)
	}

	// Serialize to JSON
	data, err := unit.ToJSON()
	if err != nil {
		return fmt.Errorf("failed to serialize Phase3RoboTorqUnit: %w", err)
	}

	// Publish with retry logic
	var lastErr error
	for attempt := 0; attempt <= p.retries; attempt++ {
		if attempt > 0 {
			// Exponential backoff: 100ms, 200ms, 400ms
			backoff := p.backoffBase * time.Duration(1<<uint(attempt-1))
			p.logger.Debug("retrying NATS publish",
				"attempt", attempt,
				"backoff_ms", backoff.Milliseconds(),
				"unit_id", unit.UnitID,
			)
			p.metrics.RetryAttemptsTotal.Inc()

			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(backoff):
				// Continue to retry
			}
		}

		// Attempt publish
		err = p.natsConn.Publish(Phase3UnitsTopic, data)
		if err == nil {
			// Success!
			duration := time.Since(startTime)
			p.metrics.PublishLatency.Observe(duration.Seconds())
			p.logger.Debug("NATS publish successful",
				"unit_id", unit.UnitID,
				"topic", Phase3UnitsTopic,
				"attempts", attempt+1,
				"duration_ms", duration.Milliseconds(),
			)
			return nil
		}

		lastErr = err
		p.logger.Warn("NATS publish attempt failed",
			"attempt", attempt+1,
			"max_attempts", p.retries+1,
			"error", err,
			"unit_id", unit.UnitID,
		)
	}

	// All retries exhausted
	duration := time.Since(startTime)
	p.metrics.PublishLatency.Observe(duration.Seconds())

	p.logger.Error("NATS publish failed after all retries",
		"unit_id", unit.UnitID,
		"attempts", p.retries+1,
		"error", lastErr,
		"duration_ms", duration.Milliseconds(),
	)

	return fmt.Errorf("failed to publish after %d attempts: %w", p.retries+1, lastErr)
}

// drainRemainingUnits publishes any units left in the channel during shutdown.
//
// This ensures no units are lost during graceful shutdown.
// Uses a 100ms timeout per iteration to avoid hanging on empty channel.
func (p *Phase3DistoDamPublisher) drainRemainingUnits(ctx context.Context) {
	p.logger.Info("Draining remaining units from channel...")

	drainCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	drained := 0
	drainTimeout := 100 * time.Millisecond // Short timeout for empty channel detection

	for {
		select {
		case unit, ok := <-p.unitChannel:
			if !ok {
				// Channel closed
				p.logger.Info("Channel drained (channel closed)",
					"units_drained", drained)
				return
			}

			// Publish unit
			if err := p.publishUnit(drainCtx, unit); err != nil {
				p.logger.Error("Failed to publish unit during drain",
					"error", err,
					"unit_id", unit.UnitID,
				)
				p.metrics.PublishErrorsTotal.Inc()
			} else {
				drained++
				p.metrics.UnitsPublishedTotal.Inc()
			}

		case <-time.After(drainTimeout):
			// No units for 100ms - assume channel is empty
			p.logger.Info("Channel drained (no more units)",
				"units_drained", drained)
			return

		case <-drainCtx.Done():
			// Overall timeout - give up
			p.logger.Warn("Drain timeout, some units may be lost",
				"units_drained", drained)
			return
		}
	}
}
