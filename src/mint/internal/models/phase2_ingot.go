// internal/models/phase2_ingot.go
// Phase 2 Ingot data model - hash-only ingots from Refinery

package models

import (
	"encoding/hex"
	"errors"
	"fmt"
	"time"
)

// Phase2Ingot represents a hash-only ingot received from Refinery (Phase 2)
// This is the input to the Mint service - contains merkle root from 3600 JTU hashes
//
// Data flow:
// - Refinery aggregates 3600 JTU hashes
// - Builds merkle tree (Level 1)
// - Sends Phase2Ingot with branch_hash (merkle root)
// - Mint receives, validates, and aggregates 1000 ingots into RoboTorqUnit
type Phase2Ingot struct {
	ID             string    `json:"id"`               // UUID from Refinery
	BranchHash     string    `json:"branch_hash"`      // Merkle root (64-char hex SHA256)
	HashCount      int       `json:"hash_count"`       // Always 3600
	ContractIDs    []string  `json:"contract_ids"`     // Unique contracts in this ingot
	DiggerIDs      []string  `json:"digger_ids"`       // Unique diggers who contributed
	RoboStakeTotal float64   `json:"robo_stake_total"` // Total RoboStake for 3600 units
	Timestamp      time.Time `json:"timestamp"`        // When assembled by Refinery

	// Phase 5: Falcon-1024 signature from Refinery (proof of assembly)
	Signature string `json:"signature"`  // Hex-encoded Falcon-1024 signature
	PublicKey string `json:"public_key"` // Hex-encoded Falcon-1024 public key

	// Phase 2: NO Units[] array - hash-only flow
	// Full JTU data remains on Digger for audit/verification
}

// Validate checks if the Phase2Ingot has valid structure
func (pi *Phase2Ingot) Validate() error {
	// Branch hash must be 64-char hex (32 bytes)
	if len(pi.BranchHash) != 64 {
		return fmt.Errorf("invalid branch_hash length: %d (expected 64)", len(pi.BranchHash))
	}

	// Verify it's valid hex
	if _, err := hex.DecodeString(pi.BranchHash); err != nil {
		return fmt.Errorf("branch_hash is not valid hex: %w", err)
	}

	// Hash count must be exactly 3600 (ingot threshold)
	if pi.HashCount != 3600 {
		return fmt.Errorf("invalid hash_count: %d (expected 3600)", pi.HashCount)
	}

	// Must have at least one contract
	if len(pi.ContractIDs) == 0 {
		return errors.New("contract_ids cannot be empty")
	}

	// Must have at least one digger
	if len(pi.DiggerIDs) == 0 {
		return errors.New("digger_ids cannot be empty")
	}

	// ID must not be empty
	if pi.ID == "" {
		return errors.New("id cannot be empty")
	}

	// Timestamp should be reasonable (not zero, not far future)
	if pi.Timestamp.IsZero() {
		return errors.New("timestamp cannot be zero")
	}

	if pi.Timestamp.After(time.Now().Add(1 * time.Hour)) {
		return fmt.Errorf("timestamp is too far in future: %s", pi.Timestamp)
	}

	// Signature must not be empty (Phase 5)
	if pi.Signature == "" {
		return errors.New("signature cannot be empty")
	}

	// Public key must not be empty (Phase 5)
	if pi.PublicKey == "" {
		return errors.New("public_key cannot be empty")
	}

	// Validate signature hex format
	if _, err := hex.DecodeString(pi.Signature); err != nil {
		return fmt.Errorf("signature is not valid hex: %w", err)
	}

	// Validate public key hex format
	if _, err := hex.DecodeString(pi.PublicKey); err != nil {
		return fmt.Errorf("public_key is not valid hex: %w", err)
	}

	return nil
}
