// internal/refinery/ingot_assembler_phase2.go
// Phase 2: Hash-only ingot assembly using merkle trees
//
// Replaces Phase 1 full-unit accumulation with hash-only aggregation.
// Ingots contain ONLY the merkle root, not full JTU data.

package refinery

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"b2b/refinery/internal/crypto"

	"github.com/google/uuid"
	"github.com/prometheus/client_golang/prometheus"
)

const (
	// HashesPerIngot is the number of hashes needed to create one ingot
	// Per RoboTorq white paper: 3600 JTUs = 1 Ingot
	HashesPerIngot = 3600
)

var (
	// Phase 2 metrics
	merkleTreesBuiltTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_merkle_trees_built_total",
		Help: "Total merkle trees built from hash batches",
	})

	merkleBuildDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "refinery_merkle_build_duration_seconds",
		Help:    "Time taken to build merkle tree from 3600 hashes",
		Buckets: []float64{0.001, 0.01, 0.1, 0.5, 1.0, 2.0, 5.0},
	})

	phase2IngotsAssembledTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_phase2_ingots_assembled_total",
		Help: "Total Phase 2 ingots assembled (hash-only, with merkle roots)",
	})
)

func init() {
	prometheus.MustRegister(merkleTreesBuiltTotal)
	prometheus.MustRegister(merkleBuildDuration)
	prometheus.MustRegister(phase2IngotsAssembledTotal)
}

// Phase2Ingot is the simplified ingot structure for Phase 2 (hash-only flow)
// Contains ONLY the merkle root and metadata - NO full JTU data
type Phase2Ingot struct {
	ID             string    `json:"id"`               // UUID
	BranchHash     string    `json:"branch_hash"`      // Merkle root (64 char hex)
	HashCount      int       `json:"hash_count"`       // Always 3600
	ContractIDs    []string  `json:"contract_ids"`     // Unique contracts
	DiggerIDs      []string  `json:"digger_ids"`       // Unique diggers
	RoboStakeTotal float64   `json:"robo_stake_total"` // Total RoboStake for 3600 units
	Timestamp      time.Time `json:"timestamp"`        // When assembled

	// Phase 5: Falcon-1024 signature (proof of assembly by Refinery)
	Signature string `json:"signature"`  // Hex-encoded Falcon-1024 signature
	PublicKey string `json:"public_key"` // Hex-encoded Falcon-1024 public key
}

// Phase2IngotAssembler builds ingots from hash queues (not full units)
type Phase2IngotAssembler struct {
	queueManager *QueueManager
	ctx          context.Context
	logger       *slog.Logger
	mu           sync.Mutex
	signer       *crypto.FalconSigner // Phase 5: Sign ingots before sending to Mint

	// Completed ingots ready to send to Mint
	completedIngots []*Phase2Ingot
}

// NewPhase2IngotAssembler creates a new Phase 2 ingot assembler
func NewPhase2IngotAssembler(ctx context.Context, qm *QueueManager, logger *slog.Logger) *Phase2IngotAssembler {
	// Initialize Falcon-1024 signer for Phase 5
	signer, err := crypto.NewFalconSigner()
	if err != nil {
		logger.Error("failed to create Falcon signer", "error", err)
		// Continue without signer (will fail later, but allows startup)
		signer = nil
	} else {
		logger.Info("Falcon-1024 signer initialized for Phase2Ingots")
	}

	return &Phase2IngotAssembler{
		queueManager:    qm,
		ctx:             ctx,
		logger:          logger,
		signer:          signer,
		completedIngots: make([]*Phase2Ingot, 0),
	}
}

// Start begins the ingot assembly loop (blocking)
// Continuously waits for 3600 hashes, builds merkle tree, creates ingot
func (ia *Phase2IngotAssembler) Start() {
	ia.logger.Info("starting Phase 2 ingot assembler",
		"hashes_per_ingot", HashesPerIngot,
	)

	for {
		select {
		case <-ia.ctx.Done():
			ia.logger.Info("Phase 2 ingot assembler shutting down")
			return

		default:
			// Get exactly 3600 hashes (blocking call)
			hashes, err := ia.queueManager.GetHashes(HashesPerIngot)

			if err != nil {
				ia.logger.Error("failed to get hashes from queue", "error", err)
				return
			}

			if len(hashes) == 0 {
				// Queue closed (shutdown)
				return
			}

			// Assemble ingot from hashes
			if err := ia.assembleIngot(hashes); err != nil {
				ia.logger.Error("failed to assemble ingot", "error", err)
				continue
			}
		}
	}
}

// assembleIngot builds a merkle tree and creates Phase2Ingot
func (ia *Phase2IngotAssembler) assembleIngot(hashEntries []HashEntry) error {
	startTime := time.Now()

	// Extract just the hash strings
	hashes := make([]string, len(hashEntries))
	for i, entry := range hashEntries {
		hashes[i] = entry.Hash
	}

	// Build merkle tree
	ia.logger.Debug("building merkle tree", "hash_count", len(hashes))

	merkleStart := time.Now()
	tree, err := BuildMerkleTree(hashes)
	merkleDuration := time.Since(merkleStart).Seconds()
	merkleBuildDuration.Observe(merkleDuration)

	if err != nil {
		return fmt.Errorf("failed to build merkle tree: %w", err)
	}

	merkleTreesBuiltTotal.Inc()

	ia.logger.Debug("merkle tree built",
		"root_hash", tree.Root,
		"height", tree.Height,
		"build_time_ms", merkleDuration*1000,
	)

	// Extract unique contract IDs, digger IDs, and sum RoboStake
	contractMap := make(map[string]bool)
	diggerMap := make(map[string]bool)
	var totalRoboStake float64
	for _, entry := range hashEntries {
		contractMap[entry.ContractID] = true
		diggerMap[entry.DiggerID] = true
		totalRoboStake += entry.RoboStakePaid
	}

	contractIDs := make([]string, 0, len(contractMap))
	for id := range contractMap {
		contractIDs = append(contractIDs, id)
	}

	diggerIDs := make([]string, 0, len(diggerMap))
	for id := range diggerMap {
		diggerIDs = append(diggerIDs, id)
	}

	// Create ingot
	ingot := &Phase2Ingot{
		ID:             uuid.New().String(),
		BranchHash:     tree.Root,
		HashCount:      len(hashes),
		ContractIDs:    contractIDs,
		DiggerIDs:      diggerIDs,
		RoboStakeTotal: totalRoboStake,
		Timestamp:      time.Now(),
	}

	// Phase 5: Sign the ingot with Falcon-1024 before sending to Mint
	if ia.signer != nil {
		timestamp := ingot.Timestamp.Format(time.RFC3339)
		signature, publicKey, err := ia.signer.SignPhase2Ingot(
			ingot.ID,
			ingot.BranchHash,
			ingot.HashCount,
			timestamp,
		)
		if err != nil {
			ia.logger.Error("failed to sign Phase2Ingot",
				"error", err,
				"ingot_id", ingot.ID)
			return fmt.Errorf("failed to sign ingot: %w", err)
		}

		ingot.Signature = signature
		ingot.PublicKey = publicKey

		ia.logger.Debug("Phase2Ingot signed",
			"ingot_id", ingot.ID,
			"signature_len", len(signature),
			"pubkey_len", len(publicKey))
	} else {
		ia.logger.Warn("no Falcon signer available - ingot NOT signed (will be rejected by Mint!)",
			"ingot_id", ingot.ID)
	}

	// Store completed ingot
	ia.mu.Lock()
	ia.completedIngots = append(ia.completedIngots, ingot)
	ia.mu.Unlock()

	phase2IngotsAssembledTotal.Inc()

	totalDuration := time.Since(startTime).Milliseconds()

	ia.logger.Info("Phase 2 ingot assembled",
		"ingot_id", ingot.ID,
		"branch_hash", ingot.BranchHash,
		"hash_count", ingot.HashCount,
		"contracts", len(ingot.ContractIDs),
		"diggers", len(ingot.DiggerIDs),
		"merkle_height", tree.Height,
		"assembly_time_ms", totalDuration,
	)

	return nil
}

// GetCompletedIngots returns all completed ingots and clears the list
func (ia *Phase2IngotAssembler) GetCompletedIngots() []*Phase2Ingot {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	ingots := make([]*Phase2Ingot, len(ia.completedIngots))
	copy(ingots, ia.completedIngots)

	// Clear the completed ingots list
	ia.completedIngots = make([]*Phase2Ingot, 0)

	return ingots
}

// GetCompletedIngotsCount returns the number of ingots ready for sending to Mint
func (ia *Phase2IngotAssembler) GetCompletedIngotsCount() int {
	ia.mu.Lock()
	defer ia.mu.Unlock()
	return len(ia.completedIngots)
}
