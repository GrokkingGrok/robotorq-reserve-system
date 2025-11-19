// Package models provides data structures for the Mint service.
package models

import "time"

// IngotHashEntry represents a hash entry extracted from a Phase2Ingot.
// This is the storage format for the IngotHashQueue - we store only the hash
// and metadata, not the full ingot, to minimize memory usage.
//
// Memory savings: ~50 bytes per entry vs ~500+ bytes for full Phase2Ingot
type IngotHashEntry struct {
	// BranchHash is the 64-character hex merkle branch hash from the ingot
	BranchHash string `json:"branch_hash"`

	// ContractIDs are the unique contract IDs that contributed to this ingot
	ContractIDs []string `json:"contract_ids"`

	// DiggerIDs are the unique digger IDs that contributed to this ingot
	DiggerIDs []string `json:"digger_ids"`

	// RoboStakeTotal is the accumulated RoboStake for this ingot (3600 units)
	RoboStakeTotal float64 `json:"robo_stake_total"`

	// RefineryID identifies which Refinery sent this ingot (future multi-refinery support)
	RefineryID string `json:"refinery_id,omitempty"`

	// Timestamp is when the ingot was created by the Refinery
	Timestamp time.Time `json:"timestamp"`
}

// NewIngotHashEntry creates an IngotHashEntry from a Phase2Ingot
func NewIngotHashEntry(ingot *Phase2Ingot) *IngotHashEntry {
	return &IngotHashEntry{
		BranchHash:     ingot.BranchHash,
		ContractIDs:    ingot.ContractIDs,
		DiggerIDs:      ingot.DiggerIDs,
		RoboStakeTotal: ingot.RoboStakeTotal,
		RefineryID:     "", // Future: extract from ingot metadata
		Timestamp:      ingot.Timestamp,
	}
}
