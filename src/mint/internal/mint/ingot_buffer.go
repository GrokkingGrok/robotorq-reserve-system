// Package mint provides the IngotBuffer component.
// IngotBuffer is a thread-safe channel-based buffer for ingots.
package mint

import (
	"context"
	"log/slog"
	"sync/atomic"

	"github.com/prometheus/client_golang/prometheus"
)

// ─────────────────────────────────────────────────────────────
// IngotBuffer Implementation
// ─────────────────────────────────────────────────────────────

// ingotBuffer implements the IngotBuffer interface.
// It uses a Go channel for thread-safe buffering and atomic counters for metrics.
type ingotBuffer struct {
	channel  chan *TokenTorqIngot
	capacity int
	depth    atomic.Int64 // Current number of ingots in buffer
	metrics  *ingotBufferMetrics
	logger   *slog.Logger
}

// ingotBufferMetrics holds Prometheus metrics for IngotBuffer.
type ingotBufferMetrics struct {
	ingotsBuffered prometheus.Gauge
	pushCount      prometheus.Counter
	popCount       prometheus.Counter
	drainCount     prometheus.Counter
}

// NewIngotBuffer creates a new thread-safe ingot buffer.
func NewIngotBuffer(capacity int, logger *slog.Logger) IngotBuffer {
	metrics := &ingotBufferMetrics{
		ingotsBuffered: prometheus.NewGauge(prometheus.GaugeOpts{
			Name: "mint_ingots_buffered",
			Help: "Current number of ingots in the buffer",
		}),
		pushCount: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_buffer_push_total",
			Help: "Total number of ingots pushed to buffer",
		}),
		popCount: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_buffer_pop_total",
			Help: "Total number of ingots popped from buffer",
		}),
		drainCount: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_buffer_drain_total",
			Help: "Total number of buffer drain operations",
		}),
	}

	// Register metrics
	prometheus.MustRegister(
		metrics.ingotsBuffered,
		metrics.pushCount,
		metrics.popCount,
		metrics.drainCount,
	)

	buffer := &ingotBuffer{
		channel:  make(chan *TokenTorqIngot, capacity),
		capacity: capacity,
		metrics:  metrics,
		logger:   logger,
	}

	// Initialize depth to 0
	buffer.depth.Store(0)

	return buffer
}

// ─────────────────────────────────────────────────────────────
// IngotBuffer Interface Implementation
// ─────────────────────────────────────────────────────────────

// Push adds an ingot to the buffer.
// Returns ErrBufferFull if buffer is at capacity.
func (b *ingotBuffer) Push(ingot *TokenTorqIngot) error {
	select {
	case b.channel <- ingot:
		// Success - update metrics
		currentDepth := b.depth.Add(1)
		b.metrics.ingotsBuffered.Set(float64(currentDepth))
		b.metrics.pushCount.Inc()

		b.logger.Debug("ingot pushed to buffer",
			"depth", currentDepth,
			"capacity", b.capacity,
			"utilization", float64(currentDepth)/float64(b.capacity)*100,
		)

		return nil

	default:
		// Channel is full
		b.logger.Warn("buffer full, cannot push ingot",
			"depth", b.depth.Load(),
			"capacity", b.capacity,
		)
		return ErrBufferFull
	}
}

// Pop retrieves the next ingot from buffer.
// Blocks until an ingot is available or context is cancelled.
func (b *ingotBuffer) Pop(ctx context.Context) (*TokenTorqIngot, error) {
	select {
	case <-ctx.Done():
		// Context cancelled
		b.logger.Debug("pop cancelled by context")
		return nil, ctx.Err()

	case ingot := <-b.channel:
		// Got an ingot - update metrics
		currentDepth := b.depth.Add(-1)
		b.metrics.ingotsBuffered.Set(float64(currentDepth))
		b.metrics.popCount.Inc()

		b.logger.Debug("ingot popped from buffer",
			"depth", currentDepth,
			"capacity", b.capacity,
		)

		return ingot, nil
	}
}

// Len returns current buffer depth (thread-safe).
func (b *ingotBuffer) Len() int {
	return int(b.depth.Load())
}

// Cap returns buffer capacity.
func (b *ingotBuffer) Cap() int {
	return b.capacity
}

// Drain empties the buffer and returns all remaining ingots.
// Used during graceful shutdown to prevent data loss.
func (b *ingotBuffer) Drain() []*TokenTorqIngot {
	b.logger.Info("draining buffer",
		"current_depth", b.depth.Load(),
	)

	var ingots []*TokenTorqIngot

	// Drain all ingots from channel
	for {
		select {
		case ingot := <-b.channel:
			ingots = append(ingots, ingot)
			b.depth.Add(-1)
		default:
			// Channel is empty
			goto done
		}
	}

done:
	// Update metrics
	b.metrics.ingotsBuffered.Set(0)
	b.metrics.drainCount.Inc()

	b.logger.Info("buffer drained",
		"ingots_drained", len(ingots),
		"final_depth", b.depth.Load(),
	)

	return ingots
}
