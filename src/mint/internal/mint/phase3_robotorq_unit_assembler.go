package mint

import (
	"context"
	"fmt"
	"log/slog"

	"b2b/mint/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// Phase3RoboTorqUnitAssembler assembles Phase 3 RoboTorq units from Level 2 merkle trees.
//
// Architecture:
//   - Consumes Level2MerkleResult from Level2MerkleBuilder
//   - Creates minimal Phase3RoboTorqUnit (merkle root only)
//   - Logs metadata (contracts, diggers, refineries) but DOES NOT persist it
//   - Returns Phase3RoboTorqUnit ready for DistoDam publishing
//
// Design Philosophy:
//   - On-chain: Store only merkle root (proof of work)
//   - Off-chain: Full metadata in proof archives (Mint + Refinery)
//   - Clean separation: Proofs in archives, roots on-chain
//
// NFC Compatibility:
//   - Phase3RoboTorqUnit: ~163 bytes (fits in NTAG215's 540 bytes)
//   - With signature overhead: ~356 bytes total
//   - Remaining: 184 bytes spare for additional data
type Phase3RoboTorqUnitAssembler struct {
	logger          *slog.Logger
	metrics         *Phase3AssemblerMetrics
	merkleBuilder   *Level2MerkleBuilder
	unitChannel     chan *models.Phase3RoboTorqUnit
	channelCapacity int
}

// Phase3AssemblerMetrics tracks Phase 3 unit assembly metrics
type Phase3AssemblerMetrics struct {
	UnitsAssembledTotal     prometheus.Counter
	AssemblyErrorsTotal     prometheus.Counter
	AssemblyDurationSeconds prometheus.Histogram
	ContractsPerUnit        prometheus.Histogram
	DiggersPerUnit          prometheus.Histogram
	RefineriesPerUnit       prometheus.Histogram
}

// NewPhase3AssemblerMetrics creates Prometheus metrics for Phase 3 assembler
func NewPhase3AssemblerMetrics(reg prometheus.Registerer) *Phase3AssemblerMetrics {
	m := &Phase3AssemblerMetrics{
		UnitsAssembledTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase3_units_assembled_total",
			Help: "Total number of Phase 3 RoboTorq units assembled",
		}),
		AssemblyErrorsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase3_assembly_errors_total",
			Help: "Total number of Phase 3 unit assembly errors",
		}),
		AssemblyDurationSeconds: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase3_assembly_duration_seconds",
			Help:    "Time to assemble Phase 3 unit from merkle result",
			Buckets: []float64{0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0},
		}),
		ContractsPerUnit: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase3_contracts_per_unit",
			Help:    "Number of unique contracts per Phase 3 unit",
			Buckets: []float64{1, 5, 10, 20, 50, 100},
		}),
		DiggersPerUnit: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase3_diggers_per_unit",
			Help:    "Number of unique diggers per Phase 3 unit",
			Buckets: []float64{1, 5, 10, 20, 50, 100},
		}),
		RefineriesPerUnit: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase3_refineries_per_unit",
			Help:    "Number of unique refineries per Phase 3 unit",
			Buckets: []float64{1, 2, 3, 5, 10},
		}),
	}

	if reg != nil {
		reg.MustRegister(
			m.UnitsAssembledTotal,
			m.AssemblyErrorsTotal,
			m.AssemblyDurationSeconds,
			m.ContractsPerUnit,
			m.DiggersPerUnit,
			m.RefineriesPerUnit,
		)
	}

	return m
}

// NewPhase3RoboTorqUnitAssembler creates a new Phase 3 unit assembler
//
// Parameters:
//   - logger: Structured logger
//   - metrics: Prometheus metrics
//   - merkleBuilder: Level 2 merkle tree builder
//   - channelCapacity: Capacity of output channel (default: 10)
//
// Returns:
//   - Phase3RoboTorqUnitAssembler ready to start
func NewPhase3RoboTorqUnitAssembler(
	logger *slog.Logger,
	metrics *Phase3AssemblerMetrics,
	merkleBuilder *Level2MerkleBuilder,
	channelCapacity int,
) *Phase3RoboTorqUnitAssembler {
	if channelCapacity <= 0 {
		channelCapacity = 10 // Default capacity
	}

	return &Phase3RoboTorqUnitAssembler{
		logger:          logger,
		metrics:         metrics,
		merkleBuilder:   merkleBuilder,
		unitChannel:     make(chan *models.Phase3RoboTorqUnit, channelCapacity),
		channelCapacity: channelCapacity,
	}
}

// Start begins assembling Phase 3 units in a background goroutine
//
// Flow:
//  1. Wait for Level2MerkleResult from merkleBuilder
//  2. Extract merkle root
//  3. Create Phase3RoboTorqUnit (merkle root only)
//  4. Log metadata (contracts, diggers, refineries) - NOT persisted
//  5. Publish to unitChannel for DistoDam
//  6. Record metrics
//
// Graceful Shutdown:
//   - Respects ctx.Done() cancellation
//   - Closes unitChannel on exit
//
// Parameters:
//   - ctx: Context for cancellation
func (a *Phase3RoboTorqUnitAssembler) Start(ctx context.Context) {
	defer close(a.unitChannel)

	a.logger.Info("phase3 assembler started",
		"channel_capacity", a.channelCapacity)

	for {
		select {
		case <-ctx.Done():
			a.logger.Info("phase3 assembler shutting down", "reason", ctx.Err())
			return

		default:
			// Build Level 2 merkle tree (blocks until 1000 ingots ready)
			merkleResult, err := a.merkleBuilder.BuildLevel2Tree(ctx)
			if err != nil {
				if ctx.Err() != nil {
					// Context cancelled during build
					a.logger.Info("phase3 assembler cancelled during merkle build")
					return
				}

				a.logger.Error("failed to build level 2 merkle tree",
					"error", err)
				a.metrics.AssemblyErrorsTotal.Inc()
				continue
			}

			// Assemble Phase 3 unit
			unit, err := a.assembleUnit(merkleResult)
			if err != nil {
				a.logger.Error("failed to assemble phase3 unit",
					"error", err,
					"merkle_root", merkleResult.MerkleRoot)
				a.metrics.AssemblyErrorsTotal.Inc()
				continue
			}

			// Publish to channel (blocks if channel full)
			select {
			case a.unitChannel <- unit:
				a.logger.Info("phase3 unit assembled and published",
					"unit_id", unit.UnitID,
					"merkle_root", unit.MerkleRoot,
					"contracts", len(merkleResult.ContractIDs),
					"diggers", len(merkleResult.DiggerIDs),
					"refineries", len(merkleResult.RefineryIDs),
					"size_bytes", unit.SizeBytes())

			case <-ctx.Done():
				a.logger.Info("phase3 assembler cancelled while publishing unit")
				return
			}
		}
	}
}

// assembleUnit creates a Phase3RoboTorqUnit from Level2MerkleResult
//
// Process:
//  1. Extract merkle root from result
//  2. Create minimal Phase3RoboTorqUnit (merkle root only)
//  3. Log metadata for observability (NOT persisted to unit)
//  4. Record metrics
//
// Parameters:
//   - merkleResult: Level 2 merkle tree result with metadata
//
// Returns:
//   - Phase3RoboTorqUnit ready for DistoDam
//   - Error if assembly fails
func (a *Phase3RoboTorqUnitAssembler) assembleUnit(merkleResult *Level2MerkleResult) (*models.Phase3RoboTorqUnit, error) {
	timer := prometheus.NewTimer(a.metrics.AssemblyDurationSeconds)
	defer timer.ObserveDuration()

	// Validate merkle result
	if merkleResult == nil {
		return nil, fmt.Errorf("merkle result is nil")
	}

	if merkleResult.MerkleRoot == "" {
		return nil, fmt.Errorf("merkle root is empty")
	}

	// Create minimal Phase 3 unit (merkle root only)
	unit, err := models.NewPhase3RoboTorqUnit(merkleResult.MerkleRoot)
	if err != nil {
		return nil, fmt.Errorf("failed to create phase3 unit: %w", err)
	}

	// Log metadata for observability (NOT persisted to unit)
	a.logger.Info("phase3 unit metadata",
		"unit_id", unit.UnitID,
		"contracts", merkleResult.ContractIDs,
		"diggers", merkleResult.DiggerIDs,
		"refineries", merkleResult.RefineryIDs,
		"tree_height", merkleResult.TreeHeight,
		"note", "metadata logged for observability, not persisted to unit")

	// Record metadata metrics
	a.metrics.ContractsPerUnit.Observe(float64(len(merkleResult.ContractIDs)))
	a.metrics.DiggersPerUnit.Observe(float64(len(merkleResult.DiggerIDs)))
	a.metrics.RefineriesPerUnit.Observe(float64(len(merkleResult.RefineryIDs)))
	a.metrics.UnitsAssembledTotal.Inc()

	return unit, nil
}

// GetUnitChannel returns the output channel for Phase 3 units
//
// Consumer (DistoDam publisher) should read from this channel:
//
//	for unit := range assembler.GetUnitChannel() {
//	    // Publish to DistoDam
//	}
//
// Returns:
//   - Read-only channel of Phase3RoboTorqUnit
func (a *Phase3RoboTorqUnitAssembler) GetUnitChannel() <-chan *models.Phase3RoboTorqUnit {
	return a.unitChannel
}
