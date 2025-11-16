package refinery

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
)

// MerkleTree represents a binary hash tree built from leaf hashes
// Used to aggregate 3600 JouleTorqUnit hashes into a single ingot hash
//
// Properties:
// - Binary tree: each parent = SHA256(left_child + right_child)
// - Deterministic: same inputs always produce same root
// - Efficient verification: log₂(N) proof size
// - Quantum-resistant: uses SHA256 (not relying on discrete log)
//
// Phase 2 Usage:
//  1. Digger sends 3600 hashes to NATS
//  2. Refinery queues hashes (no full JTU data)
//  3. Build merkle tree from 3600 hashes
//  4. Store root in TokenTorqIngot.BranchHash
//  5. Full JTU data remains on Digger for audit
type MerkleTree struct {
	Leaves []string // Original leaf hashes (input)
	Root   string   // Merkle root (final hash)
	Height int      // Tree height (log₂(leaves))
}

// BuildMerkleTree constructs a binary merkle tree from leaf hashes
//
// Algorithm:
//  1. Start with leaf hashes as bottom layer
//  2. Pair adjacent hashes: hash(left + right)
//  3. If odd number, duplicate last hash
//  4. Repeat until single root hash
//
// Example (4 leaves):
//
//	     ROOT
//	    /    \
//	  AB      CD
//	 /  \    /  \
//	A    B  C    D  <- leaves
//
// Where:
//   - AB = SHA256(A + B)
//   - CD = SHA256(C + D)
//   - ROOT = SHA256(AB + CD)
//
// For 3600 leaves: height = ⌈log₂(3600)⌉ = 12 levels
func BuildMerkleTree(hashes []string) (*MerkleTree, error) {
	if len(hashes) == 0 {
		return nil, fmt.Errorf("cannot build merkle tree from empty hash list")
	}

	tree := &MerkleTree{
		Leaves: make([]string, len(hashes)),
	}
	copy(tree.Leaves, hashes)

	// Special case: single leaf
	if len(hashes) == 1 {
		// Root = hash(leaf + leaf)
		tree.Root = hashPair(hashes[0], hashes[0])
		tree.Height = 1
		return tree, nil
	}

	// Build tree bottom-up
	currentLevel := make([]string, len(hashes))
	copy(currentLevel, hashes)

	height := 0

	// Keep combining pairs until we reach the root
	for len(currentLevel) > 1 {
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

		currentLevel = nextLevel
		height++
	}

	tree.Root = currentLevel[0]
	tree.Height = height

	return tree, nil
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

// VerifyRoot checks if the merkle root was built from the given leaves
// Returns true if BuildMerkleTree(leaves).Root == expectedRoot
func VerifyRoot(leaves []string, expectedRoot string) (bool, error) {
	tree, err := BuildMerkleTree(leaves)
	if err != nil {
		return false, err
	}

	return tree.Root == expectedRoot, nil
}

// GetProofPath returns the hashes needed to verify a leaf is in the tree
// (NOT IMPLEMENTED - future enhancement for merkle proofs)
// GenerateProof creates a merkle proof for a specific hash at index.
// Future: Implement merkle proof generation for individual unit verification.
// This will enable light clients to verify a single JTU without downloading
// the entire 3600-hash ingot.
func (tree *MerkleTree) GetProofPath(leafIndex int) ([]string, error) {
	return nil, fmt.Errorf("merkle proof generation not yet implemented")
}
