// internal/refinery/ingot_assembler.go
// Assembles TokenTorq Ingots from JouleTorq queue (3600 joule threshold)

package refinery

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"log/slog"
	"sync"

	"b2b/refinery/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

const (
	// JouleTorqThreshold is the exact amount needed to create one TokenTorq Ingot
	JouleTorqThreshold = 3600.0
)

var (
	// ingotsAssembledTotal tracks total ingots created
	ingotsAssembledTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_ingots_assembled_total",
		Help: "Total TokenTorq Ingots assembled from JouleTorq",
	})

	// ingotAssemblyDuration tracks time to assemble ingots
	ingotAssemblyDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_ingot_assembly_duration_seconds",
		Help:    "Time taken to assemble a TokenTorq Ingot",
		Buckets: prometheus.DefBuckets,
	})
)

func init() {
	prometheus.MustRegister(ingotsAssembledTotal)
	prometheus.MustRegister(ingotAssemblyDuration)
}

// IngotAssembler accumulates JouleTorq and creates TokenTorq Ingots
type IngotAssembler struct {
	queueManager *QueueManager
	ctx          context.Context
	mu           sync.Mutex

	// Accumulation state
	accumulatedJoules float64
	contractIDs       []string
	jouleHashes       []string
	roboStakeTotal    float64
	priceSum          float64 // For calculating average price
	priceCount        int     // Number of prices summed

	// Completed ingots ready for batching
	completedIngots []*models.TokenTorqIngot
}

// NewIngotAssembler creates a new ingot assembler
func NewIngotAssembler(ctx context.Context, qm *QueueManager) *IngotAssembler {
	return &IngotAssembler{
		queueManager:      qm,
		ctx:               ctx,
		accumulatedJoules: 0,
		contractIDs:       make([]string, 0),
		jouleHashes:       make([]string, 0),
		completedIngots:   make([]*models.TokenTorqIngot, 0),
	}
}

// Start begins the ingot assembly process (blocking goroutine)
func (ia *IngotAssembler) Start() {
	slog.Info("starting ingot assembler",
		"threshold", JouleTorqThreshold,
	)

	for {
		select {
		case <-ia.ctx.Done():
			slog.Info("ingot assembler shutting down")
			return

		default:
			// Get next joule item from queue (blocking)
			jouleItem, err := ia.queueManager.GetJoule()
			if err != nil {
				if err == models.ErrQueueEmpty {
					return // Shutdown initiated
				}
				slog.Error("failed to get joule from queue", "error", err)
				continue
			}

			// Get corresponding robo item (blocking)
			roboItem, err := ia.queueManager.GetRobo()
			if err != nil {
				if err == models.ErrQueueEmpty {
					return // Shutdown initiated
				}
				slog.Error("failed to get robo from queue", "error", err)
				continue
			}

			// Process the items
			if err := ia.processItems(jouleItem, roboItem); err != nil {
				slog.Error("failed to process items", "error", err)
			}
		}
	}
}

// processItems adds joule/robo to accumulator and assembles ingot if threshold met
func (ia *IngotAssembler) processItems(jouleItem *models.JouleQueueItem, roboItem *models.RoboQueueItem) error {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	timer := prometheus.NewTimer(ingotAssemblyDuration)
	defer timer.ObserveDuration()

	// Add to accumulator
	ia.accumulatedJoules += jouleItem.Amount
	ia.roboStakeTotal += roboItem.Amount
	ia.priceSum += roboItem.Price
	ia.priceCount++

	// Track contract ID (avoid duplicates)
	if !contains(ia.contractIDs, jouleItem.ContractID) {
		ia.contractIDs = append(ia.contractIDs, jouleItem.ContractID)
	}

	// Generate SHA256 hash of this joule contribution for merkle tree
	hash := ia.hashJouleItem(jouleItem)
	ia.jouleHashes = append(ia.jouleHashes, hash)

	slog.Debug("accumulated joule",
		"contract", jouleItem.ContractID,
		"amount", jouleItem.Amount,
		"total", ia.accumulatedJoules,
		"threshold", JouleTorqThreshold,
	)

	// Check if we've reached the threshold
	if ia.accumulatedJoules >= JouleTorqThreshold {
		return ia.assembleIngot()
	}

	return nil
}

// assembleIngot creates a TokenTorqIngot from accumulated joules
func (ia *IngotAssembler) assembleIngot() error {
	// Calculate average price
	avgPrice := 0.0
	if ia.priceCount > 0 {
		avgPrice = ia.priceSum / float64(ia.priceCount)
	}

	// Create copies of contract IDs and hashes for the ingot
	contractIDs := make([]string, len(ia.contractIDs))
	copy(contractIDs, ia.contractIDs)

	hashes := make([]string, len(ia.jouleHashes))
	copy(hashes, ia.jouleHashes)

	// Create the ingot using NewTokenTorqIngot constructor
	ingot := models.NewTokenTorqIngot(
		uint64(JouleTorqThreshold), // 3600 joules
		ia.roboStakeTotal,
		avgPrice,
		contractIDs,
		hashes,
	)

	// Validate the ingot
	if err := ingot.Validate(); err != nil {
		slog.Error("assembled ingot failed validation", "error", err)
		return err
	}

	// Add to completed ingots
	ia.completedIngots = append(ia.completedIngots, ingot)
	ingotsAssembledTotal.Inc()

	slog.Info("ingot assembled",
		"ingot_id", ingot.IngotID,
		"joules", ingot.JouleTorqTotal,
		"robo_stake", ingot.RoboStakeTotal,
		"price", ingot.PricePerRT,
		"contracts", len(ingot.ContractIDs),
		"hashes", len(ingot.JouleTorqHashes),
	)

	// Calculate excess joules (anything over 3600)
	excess := ia.accumulatedJoules - JouleTorqThreshold

	// Reset accumulator for next ingot
	ia.accumulatedJoules = excess
	ia.roboStakeTotal = 0
	ia.priceSum = 0
	ia.priceCount = 0
	ia.contractIDs = make([]string, 0)
	ia.jouleHashes = make([]string, 0)

	if excess > 0 {
		slog.Debug("excess joules carried over",
			"excess", excess,
		)
	}

	return nil
}

// hashJouleItem generates a SHA256 hash of the joule item for merkle tree
func (ia *IngotAssembler) hashJouleItem(item *models.JouleQueueItem) string {
	// Create a deterministic representation
	data := map[string]interface{}{
		"amount":      item.Amount,
		"contract_id": item.ContractID,
		"timestamp":   item.Timestamp.Unix(),
		"hash":        item.Hash,
	}

	// Marshal to JSON (deterministic order with sorted keys)
	jsonBytes, err := json.Marshal(data)
	if err != nil {
		slog.Error("failed to marshal joule item for hashing", "error", err)
		return ""
	}

	// Generate SHA256 hash
	hash := sha256.Sum256(jsonBytes)
	return hex.EncodeToString(hash[:])
}

// GetCompletedIngots returns all completed ingots and clears the list
func (ia *IngotAssembler) GetCompletedIngots() []*models.TokenTorqIngot {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	ingots := make([]*models.TokenTorqIngot, len(ia.completedIngots))
	copy(ingots, ia.completedIngots)

	// Clear the completed ingots list
	ia.completedIngots = make([]*models.TokenTorqIngot, 0)

	return ingots
}

// GetAccumulatedJoules returns the current accumulated joules (for health checks)
func (ia *IngotAssembler) GetAccumulatedJoules() float64 {
	ia.mu.Lock()
	defer ia.mu.Unlock()
	return ia.accumulatedJoules
}

// GetCompletedIngotsCount returns the number of ingots ready for batching
func (ia *IngotAssembler) GetCompletedIngotsCount() int {
	ia.mu.Lock()
	defer ia.mu.Unlock()
	return len(ia.completedIngots)
}

// Helper function to check if string slice contains a value
func contains(slice []string, value string) bool {
	for _, item := range slice {
		if item == value {
			return true
		}
	}
	return false
}
