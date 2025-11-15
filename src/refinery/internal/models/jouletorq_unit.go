// internal/models/jouletorq_unit.go
// JouleTorqUnit: The atomic unit of RoboTorq currency
// Represents ONE AI token's worth of verified work (joules + stake + proof)
// This is the LEAF NODE of the merkle tree

package models

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"time"
)

// TODO(currency-refactor): POST-QUANTUM CRYPTOGRAPHY
// Current: Using placeholder signature verification
// Target: Dilithium5 (NIST FIPS 204) post-quantum signatures
//
// See: src/digger-app/digger/src-tauri/src/crypto.rs for Dilithium spec
// - Signature size: ~4595 bytes (much larger than Ed25519's 64 bytes)
// - Public key: 2592 bytes
// - Security: NIST Level 5 (AES-256 equivalent, quantum-resistant)
//
// IMPLEMENTATION PLAN:
// 1. Add Go Dilithium library (e.g., go-pqcrypto or cloudflare/circl)
// 2. Replace VerifySignature() with Dilithium5 verification
// 3. Update Signature/DiggerPubKey field sizes in struct
// 4. Coordinate with Digger's crypto.rs implementation

// JouleTorqUnit represents a single joule of verified robotic work
// This is the fundamental unit of currency - the atomic proof of 1 joule
//
// Merkle Tree Structure (per RoboTorq white paper Appendix O):
// - 1 JouleTorqUnit = 1 Joule of work (merkle tree LEAF)
// - 3,600 JouleTorqUnits = 1 TokenTorqIngot (merkle tree BRANCH)
// - 1,000 TokenTorqIngots = 1 RoboTorqUnit = 1 RT (merkle tree ROOT)
// - Total: 3,600,000 JouleTorqUnits per 1 RT
//
// Each JouleTorqUnit represents:
// - 1 token/s × 1 W × 1 second = 1 Joule
// - The energy consumed to process 1 AI token output
// - A cryptographically signed proof from the Digger
type JouleTorqUnit struct {
	// Identity
	TokenID        string    `json:"token_id"`         // Unique ID: {ContractID}-{MilestoneIndex}-{TokenIndex}
	ContractID     string    `json:"contract_id"`      // Parent BRLA contract
	MilestoneIndex int       `json:"milestone_index"`  // Which milestone in contract
	TokenIndex     int       `json:"token_index"`      // Sequential token number (0-239 per ingot typically)

	// Work proof
	JoulesConsumed float64   `json:"joules_consumed"`  // Energy spent processing this token (~15J average)
	RoboStakePaid  float64   `json:"robo_stake_paid"`  // RT paid to process this token
	DiggerID       string    `json:"digger_id"`        // Executor that did the work
	Timestamp      time.Time `json:"timestamp"`        // When work was completed

	// Cryptographic proof
	Signature      string    `json:"signature"`        // Digger's Ed25519 signature over {TokenID, Joules, RoboStake, Timestamp}
	DiggerPubKey   string    `json:"digger_pub_key"`   // Digger's public key (for verification)
	Hash           string    `json:"hash"`             // SHA256 hash of this unit (merkle tree leaf)
}

// NewJouleTorqUnit creates a new unit from digger work proof
func NewJouleTorqUnit(
	contractID string,
	milestoneIndex int,
	tokenIndex int,
	joulesConsumed float64,
	roboStakePaid float64,
	diggerID string,
	signature string,
	diggerPubKey string,
) *JouleTorqUnit {
	// Generate deterministic TokenID
	tokenID := fmt.Sprintf("%s-%d-%d", contractID, milestoneIndex, tokenIndex)

	unit := &JouleTorqUnit{
		TokenID:        tokenID,
		ContractID:     contractID,
		MilestoneIndex: milestoneIndex,
		TokenIndex:     tokenIndex,
		JoulesConsumed: joulesConsumed,
		RoboStakePaid:  roboStakePaid,
		DiggerID:       diggerID,
		Timestamp:      time.Now().UTC(),
		Signature:      signature,
		DiggerPubKey:   diggerPubKey,
	}

	// Calculate hash for merkle tree
	unit.Hash = unit.CalculateHash()

	return unit
}

// CalculateHash generates the SHA256 hash of this unit (merkle tree leaf node)
// This hash reveals ALL the energy consumed to create this specific token
func (u *JouleTorqUnit) CalculateHash() string {
	// Create deterministic representation
	data := map[string]interface{}{
		"token_id":        u.TokenID,
		"contract_id":     u.ContractID,
		"milestone_index": u.MilestoneIndex,
		"token_index":     u.TokenIndex,
		"joules_consumed": u.JoulesConsumed,
		"robo_stake_paid": u.RoboStakePaid,
		"digger_id":       u.DiggerID,
		"timestamp":       u.Timestamp.Unix(),
	}

	// Marshal to JSON (sorted keys for determinism)
	jsonBytes, _ := json.Marshal(data)

	// Generate SHA256 hash
	hash := sha256.Sum256(jsonBytes)
	return hex.EncodeToString(hash[:])
}

// VerifySignature checks that the digger's Dilithium signature is valid
// TODO(currency-refactor): Replace with real Dilithium5 verification
func (u *JouleTorqUnit) VerifySignature() error {
	// STUB: Currently accepts any signature (matches Digger's stub)
	// This is INSECURE but allows us to build the data flow first
	
	// Validate fields exist
	if u.Signature == "" {
		return fmt.Errorf("missing signature for token %s", u.TokenID)
	}
	if u.DiggerPubKey == "" {
		return fmt.Errorf("missing digger public key for token %s", u.TokenID)
	}

	// TODO: Implement Dilithium5 verification
	// Example using cloudflare/circl library:
	//
	// import "github.com/cloudflare/circl/sign/dilithium/mode5"
	//
	// pubKeyBytes, err := hex.DecodeString(u.DiggerPubKey)
	// if err != nil {
	//     return fmt.Errorf("invalid public key: %w", err)
	// }
	//
	// sigBytes, err := hex.DecodeString(u.Signature)
	// if err != nil {
	//     return fmt.Errorf("invalid signature: %w", err)
	// }
	//
	// // Create message that was signed
	// message := []byte(fmt.Sprintf("%s|%.2f|%.6f|%d",
	//     u.TokenID,
	//     u.JoulesConsumed,
	//     u.RoboStakePaid,
	//     u.Timestamp.Unix(),
	// ))
	//
	// // Verify Dilithium signature
	// var pubKey mode5.PublicKey
	// if len(pubKeyBytes) != mode5.PublicKeySize {
	//     return fmt.Errorf("invalid public key size: expected %d, got %d",
	//         mode5.PublicKeySize, len(pubKeyBytes))
	// }
	// copy(pubKey[:], pubKeyBytes)
	//
	// if !mode5.Verify(&pubKey, message, sigBytes) {
	//     return fmt.Errorf("Dilithium signature verification failed for token %s", u.TokenID)
	// }

	// STUB: Always succeed (for now)
	return nil
}

// Validate checks that the unit has valid data
func (u *JouleTorqUnit) Validate() error {
	// Required fields
	if u.TokenID == "" {
		return fmt.Errorf("token_id is required")
	}
	if u.ContractID == "" {
		return fmt.Errorf("contract_id is required")
	}
	if u.DiggerID == "" {
		return fmt.Errorf("digger_id is required")
	}

	// Joules must be positive and reasonable (not millions)
	if u.JoulesConsumed <= 0 || u.JoulesConsumed > 1000 {
		return fmt.Errorf("invalid joules_consumed: %.2f (must be 0-1000J per token)", u.JoulesConsumed)
	}

	// RoboStake must be positive
	if u.RoboStakePaid < 0 {
		return fmt.Errorf("invalid robo_stake_paid: %.6f (cannot be negative)", u.RoboStakePaid)
	}

	// Timestamp must be reasonable (not in future, not ancient)
	now := time.Now().UTC()
	if u.Timestamp.After(now.Add(1 * time.Minute)) {
		return fmt.Errorf("timestamp is in the future: %s", u.Timestamp)
	}
	if u.Timestamp.Before(now.Add(-24 * time.Hour)) {
		return fmt.Errorf("timestamp is too old: %s (>24h ago)", u.Timestamp)
	}

	// Signature must exist
	if u.Signature == "" {
		return fmt.Errorf("signature is required")
	}
	if u.DiggerPubKey == "" {
		return fmt.Errorf("digger_pub_key is required")
	}

	// Verify cryptographic signature
	if err := u.VerifySignature(); err != nil {
		return fmt.Errorf("signature verification failed: %w", err)
	}

	// Hash must match
	expectedHash := u.CalculateHash()
	if u.Hash != expectedHash {
		return fmt.Errorf("hash mismatch: expected %s, got %s", expectedHash, u.Hash)
	}

	return nil
}

// TODO(currency-refactor): Future ledger integration
// When we implement the proof archive (src/mint/internal/proofs/), add:
//
// func (u *JouleTorqUnit) GetMerkleProofPath() []string {
//     // Return path from this leaf to merkle root
//     // Format: [sibling1_hash, parent_hash, ..., root_hash]
// }
//
// This will allow anyone to verify: "Does this specific token exist in the currency supply?"
// by checking the merkle proof against the published root hash.
