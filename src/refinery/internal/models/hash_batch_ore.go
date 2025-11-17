// internal/models/hash_batch_ore.go
// Data structures for hash-only ore batches from Digger (Phase 4+)

package models

import (
	"fmt"
	"time"
)

// HashBatchOre represents a batch of JTU hashes with Falcon-1024 signature from Digger
//
// Phase 4+ Format: Digger sends hash-only batches instead of full JouleTorqOre
// This reduces bandwidth by ~90% and enables cryptographic verification
//
// Flow: Digger → NATS (ore.batch) → Refinery → Verify Signature → Process Hashes
type HashBatchOre struct {
	// ContractID identifies which job/contract this work belongs to
	ContractID string `json:"contract_id"`

	// DiggerID identifies the robot that performed the work
	DiggerID string `json:"digger_id"`

	// Hashes is the array of JTU hash strings (SHA256 hex-encoded)
	// Each hash represents one JouleTorqUnit (atomic computation unit)
	Hashes []string `json:"hashes"`

	// HashCount is the number of hashes in this batch (redundant but validates integrity)
	HashCount int `json:"hash_count"`

	// Timestamp is ISO8601 timestamp when batch was created
	Timestamp string `json:"timestamp"`

	// Signature is the Falcon-1024 signature (hex-encoded)
	// Signs: contract_id|digger_id|milestone_index|joules|robo_stake|hashes_joined|timestamp
	Signature string `json:"signature"`

	// PublicKey is the Falcon-1024 public key (hex-encoded, 1793 bytes for Falcon-1024)
	PublicKey string `json:"public_key"`

	// MilestoneIndex tracks which milestone (currently always 0 for simplified batches)
	MilestoneIndex uint32 `json:"milestone_index,omitempty"`

	// Joules is energy consumed (0.0 for hash-only batches, actual value in Phase 6)
	Joules float64 `json:"joules,omitempty"`

	// RoboStake is stake amount (0.0 for hash-only batches, actual value in Phase 6)
	RoboStake float64 `json:"robo_stake,omitempty"`
}

// Validate checks if the hash batch has valid structure
func (hb *HashBatchOre) Validate() error {
	if hb.ContractID == "" {
		return fmt.Errorf("missing contract_id")
	}
	if hb.DiggerID == "" {
		return fmt.Errorf("missing digger_id")
	}
	if len(hb.Hashes) == 0 {
		return fmt.Errorf("empty hashes array")
	}
	if hb.HashCount != len(hb.Hashes) {
		return fmt.Errorf("hash_count mismatch: declared=%d actual=%d", hb.HashCount, len(hb.Hashes))
	}
	if hb.Timestamp == "" {
		return fmt.Errorf("missing timestamp")
	}
	if hb.Signature == "" {
		return fmt.Errorf("missing signature")
	}
	if hb.PublicKey == "" {
		return fmt.Errorf("missing public_key")
	}

	// Validate timestamp format
	if _, err := time.Parse(time.RFC3339, hb.Timestamp); err != nil {
		return fmt.Errorf("invalid timestamp format (expected RFC3339): %w", err)
	}

	return nil
}
