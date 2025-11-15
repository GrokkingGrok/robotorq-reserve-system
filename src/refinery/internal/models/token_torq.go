// internal/models/token_torq.go
// Data structures for TokenTorq Ingots sent to Mint

package models

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"time"
)

// TokenTorqIngot represents a completed ingot ready to be sent to Mint.
// An ingot is assembled from exactly 3,600 JouleTorqUnits (atomic token proofs)
//
// Per RoboTorq Paper (Appendix O):
// - 1 JouleTorq = 1 token/s × 1 W × 1 second (atomic unit)
// - 1 TokenTorq = 1,000 JouleTorq
// - 1 RoboTorq = 3,600 TokenTorq = 3,600,000 JouleTorq
//
// Merkle Tree Structure:
// - This is a BRANCH NODE in the merkle tree
// - Contains 3,600 leaf nodes (JouleTorqUnits)
// - BranchHash = SHA256(all unit hashes)
// - RoboTorqUnit contains 1,000 of these ingots (merkle root)
// - Total: 3,600 units × 1,000 ingots = 3,600,000 units per RT
//
// Flow: Refinery assembles ingots → sends to Mint → Mint creates RoboTorqUnit
type TokenTorqIngot struct {
	// IngotID is a unique identifier for this ingot (UUID)
	// Example: "550e8400-e29b-41d4-a716-446655440000"
	IngotID string `json:"ingot_id"`

	// Units contains exactly 3,600 JouleTorqUnits (atomic token proofs)
	// Each unit represents 1 joule of robotic work (1 token/s × 1 W × 1 s)
	// 3,600 units × 1J = 3,600J (ingot threshold)
	// This is the merkle tree branch - hash all unit hashes to get ingot hash
	Units []*JouleTorqUnit `json:"units"`

	// JouleTorqTotal is the total joules in this ingot
	// MUST always be exactly 3600 (sum of all Units[].JoulesConsumed)
	JouleTorqTotal float64 `json:"joule_torq_total"`

	// RoboStakeTotal is the accumulated RoboTorq from all units
	// Calculated as: sum(Units[].RoboStakePaid)
	RoboStakeTotal float64 `json:"robo_stake_total"`

	// ContractIDs lists all unique contracts that contributed to this ingot
	// Extracted from Units[].ContractID (deduplicated)
	// Enables tracing which jobs contributed to value creation
	ContractIDs []string `json:"contract_ids"`

	// BranchHash is the SHA256 merkle tree branch hash
	// Calculated as: hash(Units[0].Hash + Units[1].Hash + ... + Units[239].Hash)
	// This becomes a leaf in the RoboTorqUnit merkle tree
	BranchHash string `json:"branch_hash"`

	// MintedAt is when this ingot was assembled by Refinery
	// Used for auditing and time-series analysis
	MintedAt time.Time `json:"minted_at"`
}

// NewTokenTorqIngot creates a new ingot from 3,600 JouleTorqUnits
func NewTokenTorqIngot(units []*JouleTorqUnit) (*TokenTorqIngot, error) {
	if len(units) != 3600 {
		return nil, fmt.Errorf("ingot requires exactly 3,600 units, got %d", len(units))
	}

	// Calculate totals and extract contracts
	var totalJoules float64
	var totalRobo float64
	contractMap := make(map[string]bool)

	for _, unit := range units {
		totalJoules += unit.JoulesConsumed
		totalRobo += unit.RoboStakePaid
		contractMap[unit.ContractID] = true
	}

	// Extract unique contract IDs
	contractIDs := make([]string, 0, len(contractMap))
	for contractID := range contractMap {
		contractIDs = append(contractIDs, contractID)
	}

	ingot := &TokenTorqIngot{
		IngotID:        generateUUID(),
		Units:          units,
		JouleTorqTotal: totalJoules,
		RoboStakeTotal: totalRobo,
		ContractIDs:    contractIDs,
		MintedAt:       time.Now().UTC(),
	}

	// Calculate branch hash for merkle tree
	ingot.BranchHash = ingot.CalculateBranchHash()

	return ingot, nil
}

// CalculateBranchHash builds the merkle tree branch hash from all unit hashes
func (ingot *TokenTorqIngot) CalculateBranchHash() string {
	// Concatenate all 3,600 unit hashes in order
	var allHashes string
	for _, unit := range ingot.Units {
		allHashes += unit.Hash
	}

	// SHA256 of concatenated hashes = branch hash
	hash := sha256.Sum256([]byte(allHashes))
	return hex.EncodeToString(hash[:])
}

// VerifyUnit checks if a specific token exists in this ingot
func (ingot *TokenTorqIngot) VerifyUnit(tokenID string) (*JouleTorqUnit, bool) {
	for _, unit := range ingot.Units {
		if unit.TokenID == tokenID {
			return unit, true
		}
	}
	return nil, false
}

// Validate checks if the ingot has valid data
func (ingot *TokenTorqIngot) Validate() error {
	if ingot.IngotID == "" {
		return ErrInvalidIngotID
	}

	// Must have exactly 3,600 units
	if len(ingot.Units) != 3600 {
		return fmt.Errorf("ingot must have 3,600 units, got %d", len(ingot.Units))
	}

	// Validate each unit
	for i, unit := range ingot.Units {
		if err := unit.Validate(); err != nil {
			return fmt.Errorf("unit[%d] validation failed: %w", i, err)
		}
	}

	// Total joules should be ~3600 (allow small variance for floating point)
	if ingot.JouleTorqTotal < 3500 || ingot.JouleTorqTotal > 3700 {
		return fmt.Errorf("invalid joule total: %.2f (expected ~3600)", ingot.JouleTorqTotal)
	}

	// RoboStake must be positive
	if ingot.RoboStakeTotal < 0 {
		return ErrNegativeRoboStake
	}

	// Must have at least one contract
	if len(ingot.ContractIDs) == 0 {
		return ErrNoContracts
	}

	// Verify branch hash matches
	expectedHash := ingot.CalculateBranchHash()
	if ingot.BranchHash != expectedHash {
		return fmt.Errorf("branch hash mismatch: expected %s, got %s", expectedHash, ingot.BranchHash)
	}

	return nil
}

// generateUUID creates a simple UUID (will use proper library later)
// TODO: Replace with crypto/rand or google/uuid library
func generateUUID() string {
	// Placeholder - will implement proper UUID generation
	return fmt.Sprintf("ingot-%s", time.Now().Format("20060102-150405.000000"))
}
