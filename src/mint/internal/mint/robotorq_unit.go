// Package mint defines RoboTorqUnit (the merkle tree root)
package mint

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"time"
)

// TODO(currency-refactor): This file replaces the old "RoboTorqBatch" concept
// NEW NAME: RoboTorqUnit (consistent with JouleTorqUnit, TokenTorqIngot naming)
//
// MERKLE TREE STRUCTURE (3 layers):
// Layer 0 (Leaves):  JouleTorqUnit (3,600,000 units) - individual tokens (~1J each)
// Layer 1 (Branches): TokenTorqIngot (1,000 ingots) - each holds 3,600 units
// Layer 2 (Root):     RoboTorqUnit (1 unit) - merkle root = 1 RoboTorq
//
// KEY CHANGES:
// - Renamed from RoboTorqBatch → RoboTorqUnit
// - Contains 1000 TokenTorqIngots (each with 3,600 Units, not generic ingot data)
// - Calculates merkle root from ingot branch hashes using binary tree
// - Provides GetMerkleProofPath(tokenID) for verification (TODO)
// - Total value: 1 RT = 3,600,000 tokens × 1J = 3.6M joules

// RoboTorqUnit represents 1 RoboTorq worth of verified currency (merkle tree root)
// This is the final currency unit minted by the Mint service
type RoboTorqUnit struct {
	// UnitID is the unique identifier for this 1 RT unit
	UnitID string `json:"unit_id"`

	// Ingots contains exactly 1000 TokenTorqIngots
	// Each ingot = 240 JouleTorqUnits = 3600J
	// Total: 1000 × 3600J = 3.6M joules = 1 RT
	Ingots []*TokenTorqIngot `json:"ingots"`

	// TotalJoules should always equal 3,600,000 (1000 ingots × 3600J)
	TotalJoules uint64 `json:"total_joules"`

	// TotalRoboStake is the sum of all RoboTorq paid across all tokens
	TotalRoboStake float64 `json:"total_robo_stake"`

	// MerkleRoot is the SHA256 hash of all ingot branch hashes (cryptographic proof)
	// Anyone can verify: "Does token X exist?" by checking merkle path against this root
	MerkleRoot string `json:"merkle_root"`

	// MintedAt is when this RT unit was created
	MintedAt time.Time `json:"minted_at"`
}

// NewRoboTorqUnit creates a new 1 RT unit from 1000 ingots
func NewRoboTorqUnit(ingots []*TokenTorqIngot) (*RoboTorqUnit, error) {
	if len(ingots) != 1000 {
		return nil, fmt.Errorf("RoboTorqUnit requires exactly 1000 ingots, got %d", len(ingots))
	}

	// Calculate totals
	var totalJoules float64 // Changed from uint64 to match TokenTorqIngot.JouleTorqTotal
	var totalRobo float64

	for _, ingot := range ingots {
		totalJoules += ingot.JouleTorqTotal
		totalRobo += ingot.RoboStakeTotal
	}

	unit := &RoboTorqUnit{
		UnitID:         generateUnitID(),
		Ingots:         ingots,
		TotalJoules:    uint64(totalJoules), // Convert to uint64 for storage
		TotalRoboStake: totalRobo,
		MintedAt:       time.Now().UTC(),
	}

	// Calculate merkle root
	unit.MerkleRoot = unit.CalculateMerkleRoot()

	return unit, nil
}

// CalculateMerkleRoot builds the merkle tree from 1,000 ingot branch hashes
// This implements a binary merkle tree (bottom-up hash pairing)
func (u *RoboTorqUnit) CalculateMerkleRoot() string {
	if len(u.Ingots) == 0 {
		return ""
	}

	// Step 1: Extract all 1,000 ingot branch hashes (these are already merkle branches from Units)
	hashes := make([]string, len(u.Ingots))
	for i, ingot := range u.Ingots {
		hashes[i] = ingot.BranchHash
	}

	// Step 2: Build merkle tree bottom-up by repeatedly pairing and hashing
	// Example: [h0, h1, h2, h3] → [hash(h0+h1), hash(h2+h3)] → [hash(hash(h0+h1)+hash(h2+h3))]
	for len(hashes) > 1 {
		nextLevel := make([]string, 0, (len(hashes)+1)/2)

		for i := 0; i < len(hashes); i += 2 {
			if i+1 < len(hashes) {
				// Pair with next hash
				combined := hashes[i] + hashes[i+1]
				hash := sha256.Sum256([]byte(combined))
				nextLevel = append(nextLevel, hex.EncodeToString(hash[:]))
			} else {
				// Odd number: hash the last one alone (or duplicate it)
				// Bitcoin-style merkle trees duplicate the last hash
				combined := hashes[i] + hashes[i]
				hash := sha256.Sum256([]byte(combined))
				nextLevel = append(nextLevel, hex.EncodeToString(hash[:]))
			}
		}

		hashes = nextLevel
	}

	// Step 3: Return the root hash
	return hashes[0]
}

// Validate checks that the unit has valid data
func (u *RoboTorqUnit) Validate() error {
	if u.UnitID == "" {
		return fmt.Errorf("unit_id is required")
	}

	if len(u.Ingots) != 1000 {
		return fmt.Errorf("must have exactly 1000 ingots, got %d", len(u.Ingots))
	}

	// Validate each ingot has required fields
	for i, ingot := range u.Ingots {
		if ingot.IngotID == "" {
			return fmt.Errorf("ingot[%d] has empty IngotID", i)
		}
		if ingot.BranchHash == "" {
			return fmt.Errorf("ingot[%d] has empty BranchHash", i)
		}
		if len(ingot.Units) != 3600 {
			return fmt.Errorf("ingot[%d] must have 3,600 units, got %d", i, len(ingot.Units))
		}
	}

	// Verify total joules ≈ 3.6M (1000 ingots × 3600J)
	// Allow 1% variance for floating point accumulation
	expectedJoules := uint64(3_600_000)
	variance := uint64(36_000) // 1%
	if u.TotalJoules < expectedJoules-variance || u.TotalJoules > expectedJoules+variance {
		return fmt.Errorf("invalid total joules: expected ~%d, got %d", expectedJoules, u.TotalJoules)
	}

	// Verify merkle root
	expectedRoot := u.CalculateMerkleRoot()
	if u.MerkleRoot != expectedRoot {
		return fmt.Errorf("merkle root mismatch: expected %s, got %s", expectedRoot, u.MerkleRoot)
	}

	return nil
}

// VerifyIngot checks if a specific ingot exists in this unit by ID
func (u *RoboTorqUnit) VerifyIngot(ingotID string) (*TokenTorqIngot, bool) {
	for _, ingot := range u.Ingots {
		if ingot.IngotID == ingotID {
			return ingot, true
		}
	}
	return nil, false
}

// TODO(currency-refactor): Add after proof archive is implemented
// GetMerkleProofPath returns the merkle proof for a specific token
// This allows anyone to verify: "Does this token exist in this RT unit?"
// without needing the entire 240,000 token dataset
//
// func (u *RoboTorqUnit) GetMerkleProofPath(tokenID string) ([]string, error) {
//     // 1. Find which ingot contains the token
//     // 2. Get token's unit hash (leaf)
//     // 3. Get ingot's branch hash
//     // 4. Trace path from branch → root
//     // 5. Return [sibling_hashes...] for verification
// }

// VerifyTokenExists checks if a specific token is in this unit
// TODO(currency-refactor): Implement after ingots contain Units[]
//
// func (u *RoboTorqUnit) VerifyTokenExists(tokenID string) bool {
//     for _, ingot := range u.Ingots {
//         for _, unit := range ingot.Units {
//             if unit.TokenID == tokenID {
//                 return true
//             }
//         }
//     }
//     return false
// }

// generateUnitID creates a unique ID for the RT unit
func generateUnitID() string {
	// Format: RT-{timestamp}-{random}
	// TODO: Use proper UUID library
	return fmt.Sprintf("RT-%s", time.Now().Format("20060102-150405.000000"))
}

// ToJSON serializes the unit for NATS publishing
func (u *RoboTorqUnit) ToJSON() ([]byte, error) {
	return json.Marshal(u)
}

// TODO(currency-refactor): Add proof archive integration
// After implementing src/mint/internal/proofs/archive.go:
//
// func (u *RoboTorqUnit) StoreProofs(archive ProofArchive) error {
//     // Persist merkle tree to ledger
//     // Store: unit ID, merkle root, all ingot hashes, all token hashes
//     return archive.Store(u)
// }
