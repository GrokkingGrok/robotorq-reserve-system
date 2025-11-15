// internal/refinery/ingot_assembler.go
// Assembles TokenTorq Ingots from JouleTorq queue (3600 joule threshold)

// TODO(currency-refactor): COMPLETE REWRITE NEEDED
// Current problem: Dual queues (Joule + Robo) lose token granularity
// We split ore into separate queues, breaking the connection between tokens and their costs
//
// NEW DESIGN:
// 1. SINGLE QUEUE: JouleTorqUnits (not separate joule/robo)
// 2. ACCUMULATE 240 UNITS → 1 Ingot (not 3600 joules)
// 3. BUILD MERKLE TREE: Hash each unit, combine into branch hash
// 4. REMOVE QueueManager.GetJoule()/GetRobo() - replace with GetUnit()
// 5. VERIFY SIGNATURES: Reject units with invalid digger signatures
//
// KEY INSIGHT: The token is the atomic unit, not joules or RT
// Every unit carries: TokenID, JoulesConsumed, RoboStakePaid, Signature
// Ingot becomes: 240 units with verified proofs

package refinery

import (
	"context"
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

// IngotAssembler accumulates JouleTorqUnits and creates TokenTorq Ingots
type IngotAssembler struct {
	queueManager *QueueManager
	ctx          context.Context
	mu           sync.Mutex

	// Accumulation state (accumulate 3,600 units → 1 ingot)
	accumulatedUnits []*models.JouleTorqUnit

	// Completed ingots ready for batching
	completedIngots []*models.TokenTorqIngot
}

// NewIngotAssembler creates a new ingot assembler
func NewIngotAssembler(ctx context.Context, qm *QueueManager) *IngotAssembler {
	return &IngotAssembler{
		queueManager:     qm,
		ctx:              ctx,
		accumulatedUnits: make([]*models.JouleTorqUnit, 0, 3600),
		completedIngots:  make([]*models.TokenTorqIngot, 0),
	}
}

// Start begins the ingot assembly process (blocking goroutine)
func (ia *IngotAssembler) Start() {
	slog.Info("starting ingot assembler",
		"threshold", 3600,
	)

	for {
		select {
		case <-ia.ctx.Done():
			slog.Info("ingot assembler shutting down")
			return

		default:
			// Get next unit from queue (blocking)
			unit, err := ia.queueManager.GetUnit()
			if err != nil {
				if err == models.ErrQueueEmpty {
					return // Shutdown initiated
				}
				slog.Error("failed to get unit from queue", "error", err)
				continue
			}

			// Process the unit
			if err := ia.processUnit(unit); err != nil {
				slog.Error("failed to process unit", "error", err)
			}
		}
	}
}

// processUnit adds unit to accumulator and assembles ingot if 3,600 units reached
func (ia *IngotAssembler) processUnit(unit *models.JouleTorqUnit) error {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	timer := prometheus.NewTimer(ingotAssemblyDuration)
	defer timer.ObserveDuration()

	// Add unit to accumulator
	ia.accumulatedUnits = append(ia.accumulatedUnits, unit)

	slog.Debug("accumulated unit",
		"contract", unit.ContractID,
		"token_id", unit.TokenID,
		"joules", unit.JoulesConsumed,
		"total_units", len(ia.accumulatedUnits),
		"threshold", 3600,
	)

	// Check if we've reached 3,600 units (1 ingot)
	if len(ia.accumulatedUnits) >= 3600 {
		return ia.assembleIngot()
	}

	return nil
}

// assembleIngot creates a TokenTorqIngot from accumulated units
func (ia *IngotAssembler) assembleIngot() error {
	// Extract exactly 3,600 units for this ingot
	units := ia.accumulatedUnits[:3600]

	// Create the ingot using NewTokenTorqIngot constructor
	ingot, err := models.NewTokenTorqIngot(units)
	if err != nil {
		slog.Error("failed to create ingot", "error", err)
		return err
	}

	// Add to completed ingots
	ia.completedIngots = append(ia.completedIngots, ingot)
	ingotsAssembledTotal.Inc()

	// Calculate totals for logging
	var totalJoules float64
	var totalRobo float64
	contractMap := make(map[string]bool)
	for _, u := range units {
		totalJoules += u.JoulesConsumed
		totalRobo += u.RoboStakePaid
		contractMap[u.ContractID] = true
	}

	slog.Info("ingot assembled",
		"ingot_id", ingot.IngotID,
		"joules", totalJoules,
		"robo_stake", totalRobo,
		"units", len(units),
		"contracts", len(contractMap),
		"branch_hash", ingot.BranchHash,
	)

	// Keep excess units for next ingot (carry over)
	excess := ia.accumulatedUnits[3600:]
	ia.accumulatedUnits = excess

	if len(excess) > 0 {
		slog.Debug("excess units carried over",
			"excess_units", len(excess),
		)
	}

	return nil
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

// GetAccumulatedUnits returns the current number of accumulated units (for health checks)
func (ia *IngotAssembler) GetAccumulatedUnits() int {
	ia.mu.Lock()
	defer ia.mu.Unlock()
	return len(ia.accumulatedUnits)
}

// GetCompletedIngotsCount returns the number of ingots ready for batching
func (ia *IngotAssembler) GetCompletedIngotsCount() int {
	ia.mu.Lock()
	defer ia.mu.Unlock()
	return len(ia.completedIngots)
}
