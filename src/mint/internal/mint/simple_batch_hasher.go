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

// Hash processes a batch of ingots and returns aggregated data + individual stakes.
//
// PHASE 6 UPDATE: Now returns IngotStakes[] instead of summing RoboStake.
// This preserves proof chain granularity for DistoDam dual-vault architecture.
//
// Returns:
//   - batchHash: SHA256 hex string of batch data
//   - ingotStakes: array of individual stakes (preserves contract provenance)
//   - error: if batch is empty or hashing fails
func (h *simpleBatchHasher) Hash(batch []*TokenTorqIngot) (string, []IngotStake, error) {
	if len(batch) == 0 {
		return "", nil, fmt.Errorf("cannot hash empty batch")
	}

	// Sort batch by ingot branch hash for deterministic ordering
	// This ensures same batch always produces same hash
	sorted := make([]*TokenTorqIngot, len(batch))
	copy(sorted, batch)
	sort.Slice(sorted, func(i, j int) bool {
		// Sort by BranchHash (merkle tree branch hash), fallback to IngotID
		if sorted[i].BranchHash != "" && sorted[j].BranchHash != "" {
			return sorted[i].BranchHash < sorted[j].BranchHash
		}
		return sorted[i].IngotID < sorted[j].IngotID
	})

	// Build IngotStakes array (preserves granularity, not summed)
	ingotStakes := make([]IngotStake, len(sorted))

	// Concatenate ingot data for hashing
	var builder strings.Builder

	for i, ingot := range sorted {
		// Build individual IngotStake (CRITICAL: Each ingot processed separately)
		ingotStakes[i] = IngotStake{
			IngotID:        ingot.IngotID,
			RoboStakeTotal: ingot.RoboStakeTotal,
			ContractIDs:    ingot.ContractIDs,
		}

		// Build hash input: IngotID|JouleTorq|RoboStake|BranchHash|Contracts|UnitCount|MintedAt
		fmt.Fprintf(&builder, "%s|%.2f|%.6f|%s|%s|%d|%s\n",
			ingot.IngotID,
			ingot.JouleTorqTotal,
			ingot.RoboStakeTotal,
			ingot.BranchHash,
			strings.Join(ingot.ContractIDs, ","),
			len(ingot.Units),
			ingot.MintedAt.UTC().Format("2006-01-02T15:04:05.000Z"),
		)
	}

	// Compute SHA256 hash of concatenated data
	hashBytes := sha256.Sum256([]byte(builder.String()))
	batchHash := hex.EncodeToString(hashBytes[:])

	return batchHash, ingotStakes, nil
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
