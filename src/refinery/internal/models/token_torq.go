// internal/models/token_torq.go
// Data structures for TokenTorq Ingots sent to Mint

package models

import "time"

// TokenTorqIngot represents a completed ingot ready to be sent to Mint.
// An ingot is assembled from exactly 3600 JouleTorq worth of ore.
//
// Per RoboTorq Paper:
// - 1 JouleTorq = 1 token/s × 1 W × 1 second (atomic unit)
// - 1 TokenTorq = 1,000 JouleTorq
// - 1 RoboTorq = 3,600 TokenTorq = 3,600,000 JouleTorq
//
// Flow: Refinery assembles ingots → sends batches to Mint → Mint creates RoboTorq
type TokenTorqIngot struct {
	// IngotID is a unique identifier for this ingot (UUID)
	// Example: "550e8400-e29b-41d4-a716-446655440000"
	IngotID string `json:"ingot_id"`

	// JouleTorqTotal is the total joules in this ingot
	// MUST always be exactly 3600 (the threshold)
	JouleTorqTotal uint64 `json:"joule_torq"`

	// RoboStakeTotal is the accumulated RoboTorq from all contributing ore
	// Example: 0.0125 RT (sum of all robo_stake_amount from ore)
	RoboStakeTotal float64 `json:"robo_stake"`

	// PricePerRT is the average price in tokens per RoboTorq
	// Calculated as: total_tokens_generated / RoboStakeTotal
	// Example: 14,400 (180 tokens / 0.0125 RT)
	PricePerRT float64 `json:"price"`

	// ContractIDs lists all contracts that contributed to this ingot
	// Example: ["test-contract-001", "test-contract-002"]
	// Enables tracing which jobs contributed to value creation
	ContractIDs []string `json:"contract_ids"`

	// JouleTorqHashes contains SHA256 hashes of each ore contribution
	// Used for merkle tree construction in Mint
	// Example: ["a1b2c3d4...", "e5f6g7h8..."]
	// Provides cryptographic proof of ingot composition
	JouleTorqHashes []string `json:"joule_hashes"`

	// MintedAt is when this ingot was assembled by Refinery
	// Used for auditing and time-series analysis
	MintedAt time.Time `json:"minted_at"`
}

// NewTokenTorqIngot creates a new ingot with generated UUID and timestamp
func NewTokenTorqIngot(
	joules uint64,
	roboStake float64,
	price float64,
	contractIDs []string,
	hashes []string,
) *TokenTorqIngot {
	return &TokenTorqIngot{
		IngotID:         generateUUID(), // Will implement UUID generation
		JouleTorqTotal:  joules,
		RoboStakeTotal:  roboStake,
		PricePerRT:      price,
		ContractIDs:     contractIDs,
		JouleTorqHashes: hashes,
		MintedAt:        time.Now().UTC(),
	}
}

// Validate checks if the ingot has valid data
func (ingot *TokenTorqIngot) Validate() error {
	if ingot.IngotID == "" {
		return ErrInvalidIngotID
	}
	if ingot.JouleTorqTotal != 3600 {
		return ErrInvalidJouleTotal
	}
	if ingot.RoboStakeTotal < 0 {
		return ErrNegativeRoboStake
	}
	if len(ingot.ContractIDs) == 0 {
		return ErrNoContracts
	}
	if len(ingot.JouleTorqHashes) == 0 {
		return ErrNoHashes
	}
	return nil
}

// generateUUID creates a simple UUID (will use proper library later)
// TODO: Replace with crypto/rand or google/uuid library
func generateUUID() string {
	// Placeholder - will implement proper UUID generation
	return time.Now().Format("20060102-150405.000000")
}
