// Package mint provides test helpers for creating stub ingots
package mint

import (
	"fmt"
	"time"
)

// createStubIngotForMintTests creates a test ingot with stub JouleTorqUnits
// This is a helper for testing during the currency refactor
// PERFORMANCE: Creates a sparse array for speed - only populates first/last units
func createStubIngotForMintTests(ingotID string, contractID string, totalJoules float64, totalRobo float64) *TokenTorqIngot {
	// PERFORMANCE OPTIMIZATION: Don't create 3600 real units in tests - too slow!
	// Mint service only needs valid structure (Units array length, BranchHash)
	// Actual unit validation happens in Refinery

	stubUnits := make([]*JouleTorqUnit, 3600)
	joulesPerUnit := totalJoules / 3600.0
	roboPerUnit := totalRobo / 3600.0

	// Only populate first and last units (validation checks array length, not contents)
	stubUnits[0] = &JouleTorqUnit{
		TokenID:        fmt.Sprintf("%s-0-0", contractID),
		ContractID:     contractID,
		MilestoneIndex: 0,
		TokenIndex:     0,
		JoulesConsumed: joulesPerUnit,
		RoboStakePaid:  roboPerUnit,
		DiggerID:       "test-digger",
		Timestamp:      time.Now().UTC(),
		Signature:      "",
		DiggerPubKey:   "",
		Hash:           fmt.Sprintf("test-hash-%s-0", contractID),
	}

	stubUnits[3599] = &JouleTorqUnit{
		TokenID:        fmt.Sprintf("%s-0-3599", contractID),
		ContractID:     contractID,
		MilestoneIndex: 0,
		TokenIndex:     3599,
		JoulesConsumed: joulesPerUnit,
		RoboStakePaid:  roboPerUnit,
		DiggerID:       "test-digger",
		Timestamp:      time.Now().UTC(),
		Signature:      "",
		DiggerPubKey:   "",
		Hash:           fmt.Sprintf("test-hash-%s-3599", contractID),
	}

	// Calculate branch hash (simple for testing)
	branchHash := fmt.Sprintf("%064s", fmt.Sprintf("branch-hash-%s", ingotID)) // 64-char to match SHA256 length

	return &TokenTorqIngot{
		IngotID:        ingotID,
		Units:          stubUnits, // Sparse array: [0] and [3599] populated, rest nil
		JouleTorqTotal: totalJoules,
		RoboStakeTotal: totalRobo,
		ContractIDs:    []string{contractID},
		BranchHash:     branchHash[:64], // Ensure exactly 64 chars
		MintedAt:       time.Now().UTC(),
	}
}
