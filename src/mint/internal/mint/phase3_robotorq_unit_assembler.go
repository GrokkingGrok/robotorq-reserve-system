package mint

import (
	"context"
	"fmt"
	"log/slog"
	"sync"

	"b2b/mint/internal/crypto"
	"b2b/mint/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// ProofCache stores Level2MerkleResult for later proof generation
//
// Design:
//   - Thread-safe map of unitID → Level2MerkleResult
//   - Used by verification API to generate merkle proofs on-demand
//   - Contains full tree (TreeNodes) for O(log n) proof generation
//
// Lifecycle:
//   - Populated: When Phase3RoboTorqUnit assembled
//   - Queried: By GET /verify/proof/:unit_id endpoint
//   - Eviction: LRU cache (future enhancement, currently unbounded)
type ProofCache struct {
	mu      sync.RWMutex
	results map[string]*Level2MerkleResult // unitID -> merkle result
}

// NewProofCache creates a new proof cache
func NewProofCache() *ProofCache {
	return &ProofCache{
		results: make(map[string]*Level2MerkleResult),
	}
}

// Store saves a Level2MerkleResult for a given Phase3 unit
//
// Parameters:
//   - unitID: Phase3RoboTorqUnit ID (used as cache key)
//   - result: Level2MerkleResult with TreeNodes for proof generation
func (pc *ProofCache) Store(unitID string, result *Level2MerkleResult) {
	pc.mu.Lock()
	defer pc.mu.Unlock()
	pc.results[unitID] = result
}

// Get retrieves a Level2MerkleResult for a given Phase3 unit
//
// Parameters:
//   - unitID: Phase3RoboTorqUnit ID
//
// Returns:
//   - Level2MerkleResult if found
//   - nil if not found
func (pc *ProofCache) Get(unitID string) *Level2MerkleResult {
	pc.mu.RLock()
	defer pc.mu.RUnlock()
	return pc.results[unitID]
}

// Size returns the number of cached results
func (pc *ProofCache) Size() int {
	pc.mu.RLock()
	defer pc.mu.RUnlock()
	return len(pc.results)
}

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
	logger           *slog.Logger
	metrics          *Phase3AssemblerMetrics
	merkleBuilder    *Level2MerkleBuilder
	signer           *crypto.SPHINCSPlusSigner
	proofCache       *ProofCache       // Cache for merkle proof generation
	signatureArchive *SignatureArchive // NEW: Storage for SPHINCS+ signatures
	unitChannel      chan *models.Phase3RoboTorqUnit
	channelCapacity  int
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
) (*Phase3RoboTorqUnitAssembler, error) {
	if channelCapacity <= 0 {
		channelCapacity = 10 // Default capacity
	}

	// Initialize SPHINCS+ signer for archival signatures
	signer, err := crypto.NewSPHINCSPlusSigner(logger)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize SPHINCS+ signer: %w", err)
	}

	// Initialize proof cache for verification API
	proofCache := NewProofCache()

	// Initialize signature archive for SPHINCS+ signatures
	signatureArchive := NewSignatureArchive()

	a := &Phase3RoboTorqUnitAssembler{
		logger:           logger,
		metrics:          metrics,
		merkleBuilder:    merkleBuilder,
		signer:           signer,
		proofCache:       proofCache,
		signatureArchive: signatureArchive,
		unitChannel:      make(chan *models.Phase3RoboTorqUnit, channelCapacity),
		channelCapacity:  channelCapacity,
	}

	return a, nil
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
	unit, err := models.NewPhase3RoboTorqUnit(merkleResult.MerkleRoot, merkleResult.TreeHeight)
	if err != nil {
		return nil, fmt.Errorf("failed to create phase3 unit: %w", err)
	}

	// Set merkle proof API endpoint
	unit.MerkleProofAPI = fmt.Sprintf("/verify/proof/%s", unit.UnitID)

	// Store merkle result in cache for later proof generation
	a.proofCache.Store(unit.UnitID, merkleResult)
	a.logger.Debug("stored merkle result in proof cache",
		"unit_id", unit.UnitID,
		"tree_height", merkleResult.TreeHeight,
		"cache_size", a.proofCache.Size())

	// Sign unit with SPHINCS+ (archival security)
	// NOTE: Signature is NOT included in the Phase3RoboTorqUnit JSON
	// to keep it under NTAG216 limit (888 bytes). Signature is stored
	// separately in SignatureArchive and retrieved via API when needed.
	signature, publicKey, err := a.signer.SignPhase3Unit(
		unit.UnitID,
		unit.MerkleRoot,
		unit.MintedAt.Format("2006-01-02T15:04:05.000000Z07:00"),
	)
	if err != nil {
		a.logger.Warn("failed to sign phase3 unit",
			"error", err,
			"unit_id", unit.UnitID)
		// Continue without signature (will fail verification later)
	} else {
		// Store signature in archive for API retrieval
		signatureRecord := &SignatureRecord{
			UnitID:     unit.UnitID,
			Signature:  signature,
			PublicKey:  publicKey,
			MerkleRoot: unit.MerkleRoot,
			MintedAt:   unit.MintedAt.Format("2006-01-02T15:04:05.000000Z07:00"),
			SignedAt:   unit.MintedAt.Format("2006-01-02T15:04:05.000000Z07:00"),
		}

		if err := a.signatureArchive.Store(signatureRecord); err != nil {
			a.logger.Error("failed to store signature in archive",
				"error", err,
				"unit_id", unit.UnitID)
		} else {
			a.logger.Debug("phase3 unit signed and archived",
				"unit_id", unit.UnitID,
				"signature_len", len(signature),
				"public_key_len", len(publicKey),
				"archive_size", a.signatureArchive.Size())
		}
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

// GetProofCache returns the proof cache for verification API
//
// Usage:
//
//	cache := assembler.GetProofCache()
//	result := cache.Get(unitID)
//	if result != nil {
//	    proof := GetProof(result.TreeNodes, leafIndex)
//	}
//
// Returns:
//   - ProofCache instance with stored Level2MerkleResults
func (a *Phase3RoboTorqUnitAssembler) GetProofCache() *ProofCache {
	return a.proofCache
}

// GetSignatureArchive returns the signature archive for verification API
//
// Usage:
//
//	archive := assembler.GetSignatureArchive()
//	record, err := archive.Get(unitID)
//	if err == nil {
//	    // Use record.Signature and record.PublicKey for verification
//	}
//
// Returns:
//   - SignatureArchive instance with stored SPHINCS+ signatures
func (a *Phase3RoboTorqUnitAssembler) GetSignatureArchive() *SignatureArchive {
	return a.signatureArchive
}
