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
	// jouleQueueGauge tracks current joule queue size
	jouleQueueGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_joule_queue_size",
		Help: "Current number of items in the joule queue",
	})

	// roboQueueGauge tracks current robo queue size
	roboQueueGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_robo_queue_size",
		Help: "Current number of items in the robo queue",
	})

	// joulesQueuedTotal tracks total joules added to queue
	joulesQueuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_joules_queued_total",
		Help: "Total JouleTorq units queued for processing",
	})

	// roboQueuedTotal tracks total robo added to queue
	roboQueuedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_robo_queued_total",
		Help: "Total RoboTorq units queued for processing",
	})
)

func init() {
	// Register Prometheus metrics
	prometheus.MustRegister(jouleQueueGauge)
	prometheus.MustRegister(roboQueueGauge)
	prometheus.MustRegister(joulesQueuedTotal)
	prometheus.MustRegister(roboQueuedTotal)
}

// QueueManager handles buffered queues for joules and RoboStake
type QueueManager struct {
	jouleQueue chan *models.JouleQueueItem
	roboQueue  chan *models.RoboQueueItem
	ctx        context.Context
	mu         sync.RWMutex
}

// NewQueueManager creates a new queue manager with specified capacities
func NewQueueManager(ctx context.Context, jouleCapacity, roboCapacity int) *QueueManager {
	slog.Info("initializing queue manager",
		"joule_capacity", jouleCapacity,
		"robo_capacity", roboCapacity,
	)

	return &QueueManager{
		jouleQueue: make(chan *models.JouleQueueItem, jouleCapacity),
		roboQueue:  make(chan *models.RoboQueueItem, roboCapacity),
		ctx:        ctx,
	}
}

// AddJoule adds a joule item to the queue (non-blocking with backpressure)
func (qm *QueueManager) AddJoule(item *models.JouleQueueItem) error {
	select {
	case qm.jouleQueue <- item:
		// Update metrics
		joulesQueuedTotal.Add(item.Amount)
		jouleQueueGauge.Set(float64(len(qm.jouleQueue)))

		slog.Debug("joule added to queue",
			"contract", item.ContractID,
			"amount", item.Amount,
			"queue_size", len(qm.jouleQueue),
		)
		return nil

	case <-qm.ctx.Done():
		return models.ErrQueueShuttingDown

	default:
		// Queue is full, reject with backpressure
		slog.Warn("joule queue full, rejecting item",
			"contract", item.ContractID,
			"amount", item.Amount,
			"queue_size", len(qm.jouleQueue),
		)
		return models.ErrQueueFull
	}
}

// AddRobo adds a robo item to the queue (non-blocking with backpressure)
func (qm *QueueManager) AddRobo(item *models.RoboQueueItem) error {
	select {
	case qm.roboQueue <- item:
		// Update metrics
		roboQueuedTotal.Add(item.Amount)
		roboQueueGauge.Set(float64(len(qm.roboQueue)))

		slog.Debug("robo added to queue",
			"contract", item.ContractID,
			"amount", item.Amount,
			"price", item.Price,
			"queue_size", len(qm.roboQueue),
		)
		return nil

	case <-qm.ctx.Done():
		return models.ErrQueueShuttingDown

	default:
		// Queue is full, reject with backpressure
		slog.Warn("robo queue full, rejecting item",
			"contract", item.ContractID,
			"amount", item.Amount,
			"queue_size", len(qm.roboQueue),
		)
		return models.ErrQueueFull
	}
}

// GetJoule retrieves a joule item from the queue (blocking until available)
func (qm *QueueManager) GetJoule() (*models.JouleQueueItem, error) {
	select {
	case item := <-qm.jouleQueue:
		jouleQueueGauge.Set(float64(len(qm.jouleQueue)))
		return item, nil
	case <-qm.ctx.Done():
		return nil, models.ErrQueueEmpty
	}
}

// GetRobo retrieves a robo item from the queue (blocking until available)
func (qm *QueueManager) GetRobo() (*models.RoboQueueItem, error) {
	select {
	case item := <-qm.roboQueue:
		roboQueueGauge.Set(float64(len(qm.roboQueue)))
		return item, nil
	case <-qm.ctx.Done():
		return nil, models.ErrQueueEmpty
	}
}

// GetJouleQueueSize returns current joule queue size (thread-safe)
func (qm *QueueManager) GetJouleQueueSize() int {
	qm.mu.RLock()
	defer qm.mu.RUnlock()
	return len(qm.jouleQueue)
}

// GetRoboQueueSize returns current robo queue size (thread-safe)
func (qm *QueueManager) GetRoboQueueSize() int {
	qm.mu.RLock()
	defer qm.mu.RUnlock()
	return len(qm.roboQueue)
}

// GetJouleCapacity returns the total capacity of the joule queue
func (qm *QueueManager) GetJouleCapacity() int {
	return cap(qm.jouleQueue)
}

// GetRoboCapacity returns the total capacity of the robo queue
func (qm *QueueManager) GetRoboCapacity() int {
	return cap(qm.roboQueue)
}

// Close gracefully shuts down the queue manager
func (qm *QueueManager) Close() {
	slog.Info("closing queue manager",
		"joule_remaining", len(qm.jouleQueue),
		"robo_remaining", len(qm.roboQueue),
	)

	qm.mu.Lock()
	defer qm.mu.Unlock()

	// Close channels to signal no more items will be added
	close(qm.jouleQueue)
	close(qm.roboQueue)
}
