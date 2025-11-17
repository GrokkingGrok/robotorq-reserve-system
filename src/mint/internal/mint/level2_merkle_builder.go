package mint

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log/slog"

	"b2b/mint/internal/models"

	"github.com/prometheus/client_golang/prometheus"
)

// Level2MerkleBuilder builds Level 2 merkle trees from 1000 ingot hashes
//
// Architecture:
//   - Consumes 1000 IngotHashEntries from IngotHashQueue
//   - Extracts BranchHash from each entry (Level 1 ingot merkle roots)
//   - Builds binary merkle tree from 1000 hashes
//   - Returns Level 2 merkle root (aggregates 1000 ingots)
//
// Tree Structure:
//
//	Level 2 Root (1 hash)
//	    /              \
//	  ...              ...
//	   |                |
//	Level 1 Roots (1000 hashes from ingots)
//	   |                |
//	  ...  (3.6M JouleTorqUnit hashes aggregated)
//
// Performance:
//   - 1000 ingots × 3600 units = 3,600,000 total units represented
//   - Tree height: ⌈log₂(1000)⌉ = 10 levels
//   - Build time: <20ms for 1000 hashes
//
// Concurrency:
//   - Runs in dedicated goroutine
//   - Blocks on queue.GetIngotHashes(ctx) until batch ready
//   - Context cancellation for graceful shutdown
type Level2MerkleBuilder struct {
	queue   *IngotHashQueue
	logger  *slog.Logger
	metrics *Level2MerkleMetrics
}

// Level2MerkleMetrics tracks merkle tree building metrics
type Level2MerkleMetrics struct {
	TreesBuiltTotal       prometheus.Counter
	BuildDurationSeconds  prometheus.Histogram
	TreeHeightGauge       prometheus.Gauge
	InvalidHashesTotal    prometheus.Counter
	ContextCancelledTotal prometheus.Counter
}

// NewLevel2MerkleBuilder creates a new Level 2 merkle tree builder
func NewLevel2MerkleBuilder(queue *IngotHashQueue, logger *slog.Logger) *Level2MerkleBuilder {
	return &Level2MerkleBuilder{
		queue:   queue,
		logger:  logger,
		metrics: initLevel2MerkleMetrics(),
	}
}

// initLevel2MerkleMetrics initializes Prometheus metrics for Level 2 merkle builder
func initLevel2MerkleMetrics() *Level2MerkleMetrics {
	return &Level2MerkleMetrics{
		TreesBuiltTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "level2_merkle_trees_built_total",
			Help: "Total number of Level 2 merkle trees built",
		}),
		BuildDurationSeconds: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "level2_merkle_build_duration_seconds",
			Help:    "Duration of Level 2 merkle tree building in seconds",
			Buckets: prometheus.LinearBuckets(0.001, 0.005, 10), // 1ms to 50ms
		}),
		TreeHeightGauge: prometheus.NewGauge(prometheus.GaugeOpts{
			Name: "level2_merkle_tree_height",
			Help: "Height of the last built Level 2 merkle tree",
		}),
		InvalidHashesTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "level2_merkle_invalid_hashes_total",
			Help: "Total number of invalid hashes encountered during tree building",
		}),
		ContextCancelledTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "level2_merkle_context_cancelled_total",
			Help: "Total number of times merkle building was cancelled",
		}),
	}
}

// Level2MerkleResult contains the result of building a Level 2 merkle tree
type Level2MerkleResult struct {
	MerkleRoot  string                   // Level 2 merkle root (64-char hex SHA256)
	HashEntries []*models.IngotHashEntry // Original 1000 hash entries
	TreeHeight  int                      // Tree height (should be 10 for 1000 hashes)
	TreeNodes   [][]string               // All tree levels for proof generation (nodes[0] = leaves, nodes[height-1] = root)
	ContractIDs []string                 // Unique contract IDs from all entries
	DiggerIDs   []string                 // Unique digger IDs from all entries
	RefineryIDs []string                 // Unique refinery IDs from all entries
}

// BuildLevel2Tree blocks until 1000 hashes are available, then builds merkle tree
//
// Returns:
//   - Level2MerkleResult with merkle root and metadata
//   - Error if context cancelled or hash validation fails
//
// Blocking:
//   - Calls queue.GetIngotHashes(ctx) which blocks until 1000 entries ready
//   - Returns immediately if context cancelled
func (b *Level2MerkleBuilder) BuildLevel2Tree(ctx context.Context) (*Level2MerkleResult, error) {
	// Block until 1000 hash entries available
	entries, err := b.queue.GetIngotHashes(ctx)
	if err != nil {
		if ctx.Err() != nil {
			b.metrics.ContextCancelledTotal.Inc()
			return nil, fmt.Errorf("context cancelled while waiting for hashes: %w", err)
		}
		return nil, fmt.Errorf("failed to get hash batch from queue: %w", err)
	}

	b.logger.Info("building Level 2 merkle tree",
		"hash_count", len(entries))

	// Extract branch hashes (Level 1 merkle roots)
	branchHashes := make([]string, len(entries))
	for i, entry := range entries {
		// Validate hash format (64-char hex)
		if len(entry.BranchHash) != 64 {
			b.metrics.InvalidHashesTotal.Inc()
			return nil, fmt.Errorf("invalid branch_hash length at index %d: got %d, expected 64",
				i, len(entry.BranchHash))
		}

		branchHashes[i] = entry.BranchHash
	}

	// Build merkle tree
	start := prometheus.NewTimer(b.metrics.BuildDurationSeconds)
	merkleRoot, height, treeNodes, err := buildMerkleTree(branchHashes)
	start.ObserveDuration()

	if err != nil {
		return nil, fmt.Errorf("failed to build merkle tree: %w", err)
	}

	b.metrics.TreesBuiltTotal.Inc()
	b.metrics.TreeHeightGauge.Set(float64(height))

	// Aggregate metadata (unique contract/digger/refinery IDs)
	result := &Level2MerkleResult{
		MerkleRoot:  merkleRoot,
		HashEntries: entries,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		ContractIDs: extractUniqueIDs(entries, func(e *models.IngotHashEntry) []string { return e.ContractIDs }),
		DiggerIDs:   extractUniqueIDs(entries, func(e *models.IngotHashEntry) []string { return e.DiggerIDs }),
		RefineryIDs: extractUniqueStrings(entries, func(e *models.IngotHashEntry) string { return e.RefineryID }),
	}

	b.logger.Info("Level 2 merkle tree built",
		"merkle_root", merkleRoot,
		"tree_height", height,
		"hash_count", len(entries),
		"contracts", len(result.ContractIDs),
		"diggers", len(result.DiggerIDs),
		"refineries", len(result.RefineryIDs))

	return result, nil
}

// buildMerkleTree constructs a binary merkle tree from hashes
//
// Algorithm:
//  1. Start with leaf hashes as bottom layer
//  2. Pair adjacent hashes: SHA256(left + right)
//  3. If odd number, duplicate last hash
//  4. Repeat until single root hash
//
// Returns: (merkle_root, tree_height, tree_nodes, error)
// tree_nodes[0] = leaves, tree_nodes[height-1] = root
func buildMerkleTree(hashes []string) (string, int, [][]string, error) {
	if len(hashes) == 0 {
		return "", 0, nil, fmt.Errorf("cannot build merkle tree from empty hash list")
	}

	// Store all tree levels for proof generation
	var allLevels [][]string

	// Special case: single hash
	if len(hashes) == 1 {
		root := hashPair(hashes[0], hashes[0])
		leafLevel := make([]string, 1)
		copy(leafLevel, hashes)
		rootLevel := []string{root}
		allLevels = [][]string{leafLevel, rootLevel}
		return root, 1, allLevels, nil
	}

	// Build tree bottom-up
	currentLevel := make([]string, len(hashes))
	copy(currentLevel, hashes)
	allLevels = append(allLevels, currentLevel) // Level 0 = leaves

	height := 0

	// Keep combining pairs until we reach the root
	for len(currentLevel) > 1 {
		height++
		nextLevel := make([]string, 0, (len(currentLevel)+1)/2)

		for i := 0; i < len(currentLevel); i += 2 {
			var combinedHash string

			if i+1 < len(currentLevel) {
				// Pair exists: hash(left + right)
				combinedHash = hashPair(currentLevel[i], currentLevel[i+1])
			} else {
				// Odd number: duplicate last hash
				combinedHash = hashPair(currentLevel[i], currentLevel[i])
			}

			nextLevel = append(nextLevel, combinedHash)
		}

		allLevels = append(allLevels, nextLevel)
		currentLevel = nextLevel
	}

	return currentLevel[0], height, allLevels, nil
}

// hashPair combines two hashes and returns SHA256(left + right)
func hashPair(left, right string) string {
	// Concatenate hex strings
	combined := left + right

	// SHA256 hash
	hash := sha256.Sum256([]byte(combined))

	// Return hex-encoded hash
	return hex.EncodeToString(hash[:])
}

// extractUniqueIDs extracts unique IDs from entries using the provided extractor function
func extractUniqueIDs(entries []*models.IngotHashEntry, extractor func(*models.IngotHashEntry) []string) []string {
	seen := make(map[string]bool)
	var unique []string

	for _, entry := range entries {
		for _, id := range extractor(entry) {
			if !seen[id] {
				seen[id] = true
				unique = append(unique, id)
			}
		}
	}

	return unique
}

// extractUniqueStrings extracts unique strings from entries using the provided extractor function
func extractUniqueStrings(entries []*models.IngotHashEntry, extractor func(*models.IngotHashEntry) string) []string {
	seen := make(map[string]bool)
	var unique []string

	for _, entry := range entries {
		id := extractor(entry)
		if !seen[id] {
			seen[id] = true
			unique = append(unique, id)
		}
	}

	return unique
}

// GetProof generates a merkle proof for a specific ingot hash index
//
// A merkle proof is the list of sibling hashes needed to reconstruct
// the path from leaf to root. This allows verification that a specific
// ingot exists in the tree without downloading all 1000 ingots.
//
// Example (4 leaves, proving index 2 = "C"):
//
//	     ROOT
//	    /    \
//	  AB      CD     ← Need AB (sibling of CD)
//	 /  \    /  \
//	A    B  C*   D   ← Need D (sibling of C)
//
// Proof for C: [D, AB]
//
// Verification:
//  1. Hash(C + D) = CD
//  2. Hash(AB + CD) = ROOT ✓
//
// Returns:
//   - proof: Array of sibling hashes (bottom to top)
//   - error: If leafIndex out of bounds
func (r *Level2MerkleResult) GetProof(leafIndex int) ([]string, error) {
	if leafIndex < 0 || leafIndex >= len(r.HashEntries) {
		return nil, fmt.Errorf("leaf index %d out of bounds (0-%d)", leafIndex, len(r.HashEntries)-1)
	}

	if len(r.TreeNodes) == 0 {
		return nil, fmt.Errorf("tree nodes not available for proof generation")
	}

	proof := make([]string, 0, r.TreeHeight-1)
	index := leafIndex

	// Walk up the tree, collecting sibling hashes
	for level := 0; level < len(r.TreeNodes)-1; level++ {
		nodes := r.TreeNodes[level]

		var sibling string
		if index%2 == 0 {
			// Left node - sibling is right
			if index+1 < len(nodes) {
				sibling = nodes[index+1]
			} else {
				// Odd number of nodes, duplicate self
				sibling = nodes[index]
			}
		} else {
			// Right node - sibling is left
			sibling = nodes[index-1]
		}

		proof = append(proof, sibling)
		index = index / 2 // Move to parent index
	}

	return proof, nil
}

// VerifyProof verifies that an ingot hash is part of the merkle tree
//
// Given:
//   - leafHash: The ingot's branch_hash to verify
//   - proof: Sibling hashes from GetProof()
//   - root: Expected merkle root (from Level2MerkleResult.MerkleRoot)
//   - leafIndex: Position in original 1000 ingots
//
// Algorithm:
//  1. Start with leafHash (ingot's branch_hash)
//  2. For each sibling in proof:
//     - Determine position (left/right based on leafIndex)
//     - Hash current with sibling
//     - Move up one level
//  3. Compare final hash with expected root
//
// Returns:
//   - true if proof is valid (reconstructed root == expected root)
//   - false if proof is invalid
func VerifyProof(leafHash string, proof []string, root string, leafIndex int) bool {
	current := leafHash
	index := leafIndex

	// Reconstruct path to root
	for _, sibling := range proof {
		if index%2 == 0 {
			// We're on the left, sibling on right
			current = hashPair(current, sibling)
		} else {
			// We're on the right, sibling on left
			current = hashPair(sibling, current)
		}
		index = index / 2
	}

	return current == root
}
