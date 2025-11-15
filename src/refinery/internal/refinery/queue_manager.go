// internal/refinery/queue_manager.go
// Manages joule and RoboStake queues with thread-safe operations

package refinery

import (
	"context"
	"log/slog"
	"sync"

	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

var (
	// unitQueueGauge tracks current JouleTorqUnit queue size
	unitQueueGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_unit_queue_size",
		Help: "Current number of JouleTorqUnits in the queue",
	})

	// unitsQueuedTotal tracks total units added to queue
	unitsQueuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_units_queued_total",
		Help: "Total JouleTorqUnits queued for processing",
	})

	// joulesQueuedTotal tracks cumulative joules queued
	joulesQueuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_joules_queued_total",
		Help: "Total joules from queued units",
	})
)

func init() {
	// Register Prometheus metrics
	prometheus.MustRegister(unitQueueGauge)
	prometheus.MustRegister(unitsQueuedTotal)
	prometheus.MustRegister(joulesQueuedTotal)
}

// QueueManager handles buffered queue for JouleTorqUnits
type QueueManager struct {
	unitQueue chan *models.JouleTorqUnit
	ctx       context.Context
	mu        sync.RWMutex
}

// NewQueueManager creates a new queue manager with specified capacity
func NewQueueManager(ctx context.Context, capacity int) *QueueManager {
	slog.Info("initializing queue manager",
		"unit_capacity", capacity,
	)

	return &QueueManager{
		unitQueue: make(chan *models.JouleTorqUnit, capacity),
		ctx:       ctx,
	}
}

// AddUnit adds a JouleTorqUnit to the queue (non-blocking with backpressure)
func (qm *QueueManager) AddUnit(unit *models.JouleTorqUnit) error {
	select {
	case qm.unitQueue <- unit:
		// Update metrics
		unitsQueuedTotal.Inc()
		joulesQueuedTotal.Add(unit.JoulesConsumed)
		unitQueueGauge.Set(float64(len(qm.unitQueue)))

		slog.Debug("unit added to queue",
			"contract", unit.ContractID,
			"token_id", unit.TokenID,
			"joules", unit.JoulesConsumed,
			"queue_size", len(qm.unitQueue),
		)
		return nil

	case <-qm.ctx.Done():
		return models.ErrQueueShuttingDown

	default:
		// Queue is full, reject with backpressure
		slog.Warn("unit queue full, rejecting item",
			"contract", unit.ContractID,
			"token_id", unit.TokenID,
			"queue_size", len(qm.unitQueue),
		)
		return models.ErrQueueFull
	}
}

// GetUnit retrieves a JouleTorqUnit from the queue (blocking until available)
func (qm *QueueManager) GetUnit() (*models.JouleTorqUnit, error) {
	select {
	case unit := <-qm.unitQueue:
		unitQueueGauge.Set(float64(len(qm.unitQueue)))
		return unit, nil
	case <-qm.ctx.Done():
		return nil, models.ErrQueueEmpty
	}
}

// GetQueueSize returns current queue size (thread-safe)
func (qm *QueueManager) GetQueueSize() int {
	qm.mu.RLock()
	defer qm.mu.RUnlock()
	return len(qm.unitQueue)
}

// GetCapacity returns the total capacity of the queue
func (qm *QueueManager) GetCapacity() int {
	return cap(qm.unitQueue)
}

// Close gracefully shuts down the queue manager
func (qm *QueueManager) Close() {
	slog.Info("closing queue manager",
		"units_remaining", len(qm.unitQueue),
	)

	qm.mu.Lock()
	defer qm.mu.Unlock()

	// Close channel to signal no more items will be added
	close(qm.unitQueue)
}
