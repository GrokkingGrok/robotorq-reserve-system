// Package mint provides batch hashing implementations.
// SimpleBatchHasher is the initial implementation using SHA256.
// Future upgrade path: FullMerkleBuilder for complete Merkle tree with proofs.
package mint

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
	"strings"
)

// simpleBatchHasher implements BatchHasher using SHA256.
//
// Algorithm:
//  1. Sort ingots by Hash (deterministic ordering)
//  2. Concatenate ingot data: Hash|JouleTorq|RoboTorq|Price
//  3. Compute SHA256 of concatenated string
//  4. Calculate totalRobo = sum(ingot.RoboTorq)
//  5. Calculate totalSale = sum(ingot.Price)
//
// Upgrade Path:
// Replace with FullMerkleBuilder that:
//   - Builds complete Merkle tree from ingot hashes
//   - Returns Merkle root as batchHash
//   - Optionally includes Merkle proofs in MintEvent
//   - Provides cryptographic verifiability for each ingot
type simpleBatchHasher struct{}

// NewSimpleBatchHasher creates a new SimpleBatchHasher instance.
func NewSimpleBatchHasher() BatchHasher {
	return &simpleBatchHasher{}
}

// Hash processes a batch of ingots and returns aggregated data.
//
// Returns:
//   - batchHash: SHA256 hex string of batch data
//   - totalRobo: sum of all ingot.RoboTorq values
//   - totalSale: sum of all ingot.Price values
//   - error: if batch is empty or hashing fails
func (h *simpleBatchHasher) Hash(batch []*TokenTorqIngot) (string, float64, float64, error) {
	if len(batch) == 0 {
		return "", 0, 0, fmt.Errorf("cannot hash empty batch")
	}

	// Sort batch by ingot hash for deterministic ordering
	// This ensures same batch always produces same hash
	sorted := make([]*TokenTorqIngot, len(batch))
	copy(sorted, batch)
	sort.Slice(sorted, func(i, j int) bool {
		return sorted[i].Hash < sorted[j].Hash
	})

	// Concatenate ingot data for hashing
	var builder strings.Builder
	var totalRobo float64
	var totalSale float64

	for _, ingot := range sorted {
		// Accumulate totals
		totalRobo += ingot.RoboTorq
		totalSale += ingot.Price

		// Build hash input: Hash|Joule|Robo|Price|ContractID|DiggerID|Timestamp
		// Include all fields for complete data integrity
		fmt.Fprintf(&builder, "%s|%.2f|%.2f|%.2f|%s|%s|%s\n",
			ingot.Hash,
			ingot.JouleTorq,
			ingot.RoboTorq,
			ingot.Price,
			ingot.ContractID,
			ingot.DiggerID,
			ingot.Timestamp.UTC().Format("2006-01-02T15:04:05.000Z"),
		)
	}

	// Compute SHA256 hash of concatenated data
	hashBytes := sha256.Sum256([]byte(builder.String()))
	batchHash := hex.EncodeToString(hashBytes[:])

	return batchHash, totalRobo, totalSale, nil
}

// ─────────────────────────────────────────────────────────────
// Future: FullMerkleBuilder Implementation
// ─────────────────────────────────────────────────────────────
//
// Design for future Merkle tree implementation:
//
// type fullMerkleBuilder struct {
//     hashAlgo crypto.Hash // SHA256, SHA3-256, etc.
// }
//
// func (h *fullMerkleBuilder) Hash(batch []*TokenTorqIngot) (string, float64, float64, error) {
//     // 1. Create leaf nodes from ingot hashes
//     leaves := make([][]byte, len(batch))
//     for i, ingot := range batch {
//         leaves[i] = []byte(ingot.Hash)
//     }
//
//     // 2. Build Merkle tree bottom-up
//     tree := buildMerkleTree(leaves, h.hashAlgo)
//
//     // 3. Extract Merkle root as batch hash
//     merkleRoot := hex.EncodeToString(tree.Root())
//
//     // 4. Calculate totals (same as SimpleBatchHasher)
//     totalRobo, totalSale := calculateTotals(batch)
//
//     // 5. Optionally store proofs for later verification
//     // proofs := tree.GenerateProofs()
//
//     return merkleRoot, totalRobo, totalSale, nil
// }
//
// Advantages of Merkle tree approach:
//  - Cryptographic proof that each ingot was included in batch
//  - Efficient verification without reconstructing entire batch
//  - Individual ingot proofs for auditing
//  - Industry-standard approach for blockchain/distributed systems
//
// Migration path:
//  1. Implement FullMerkleBuilder with BatchHasher interface
//  2. Add Merkle proof fields to MintEvent (optional, backward compatible)
//  3. Update main.go to use NewFullMerkleBuilder instead of NewSimpleBatchHasher
//  4. No changes needed to MintEngine, BatchAggregator, or other components!
