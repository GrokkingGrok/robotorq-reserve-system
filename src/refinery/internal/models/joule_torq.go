// internal/models/joule_torq.go
// Data structures for JouleTorqOre received from Digger

package models

// JouleTorqOre represents the ore payload sent by Digger after each milestone.
// This structure MUST match Digger's Rust JouleTorqOre struct exactly.
//
// Flow: Digger generates ore → sends via POST /receive-ore → Refinery receives this
type JouleTorqOre struct {
	// DiggerID identifies the robot that performed the work
	// Example: "dig-jon-ai-001"
	DiggerID string `json:"digger_id"`

	// ContractID identifies which job/contract this work belongs to
	// Example: "test-contract-001"
	ContractID string `json:"contract_id"`

	// TokensGenerated is the number of AI tokens (computation) performed
	// Example: 60 tokens per milestone
	TokensGenerated uint64 `json:"tokens_generated"`

	// Joules is the energy consumed in joules (1 watt-second)
	// Example: 1250 joules per 5-second milestone at 250W
	Joules uint64 `json:"joules"`

	// MilestoneIndex tracks which milestone this is (0-indexed)
	// Example: milestone 0, 1, 2, ... for the contract duration
	MilestoneIndex uint32 `json:"milestone_index"`

	// Timestamp is Unix timestamp (seconds since epoch) when ore was generated
	Timestamp uint64 `json:"timestamp"`

	// ProofOfWork is optional cryptographic proof of computation
	// Currently unused (stub mode), will contain base64-encoded proof in future
	ProofOfWork *string `json:"proof_of_work,omitempty"`

	// RoboStakeAmount is the fractional RoboTorq (RT) allocated to this milestone
	// Example: 0.00416 RT (from 5 RT / 1200 total milestones)
	RoboStakeAmount float64 `json:"robo_stake_amount"`

	// Signature is optional post-quantum cryptographic signature (Dilithium)
	// None in stub mode, present when crypto is enabled
	// Proves authenticity and prevents tampering
	Signature []byte `json:"signature,omitempty"`
}

// Validate checks if the ore structure has valid data
func (ore *JouleTorqOre) Validate() error {
	if ore.DiggerID == "" {
		return ErrInvalidDiggerID
	}
	if ore.ContractID == "" {
		return ErrInvalidContractID
	}
	if ore.Joules == 0 {
		return ErrZeroJoules
	}
	if ore.RoboStakeAmount < 0 {
		return ErrNegativeRoboStake
	}
	if ore.Timestamp == 0 {
		return ErrZeroTimestamp
	}
	return nil
}

// CalculatePrice computes the price ratio (tokens per RoboTorq)
// Returns 0.0 if RoboStakeAmount is zero to avoid division by zero
func (ore *JouleTorqOre) CalculatePrice() float64 {
	if ore.RoboStakeAmount == 0 {
		return 0.0
	}
	return float64(ore.TokensGenerated) / ore.RoboStakeAmount
}
