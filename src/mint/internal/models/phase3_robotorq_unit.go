package models

import (
	"encoding/json"
	"fmt"
	"time"
)

// Phase3RoboTorqUnit represents 1 RoboTorq unit using pure hash-only proof chain
//
// Architecture (Phase 3 - Pure Hash-Only):
//   - Absolute minimum: merkle root + timestamp only
//   - Size: ~150 bytes (fits on NFC tag with room for signature)
//   - Proof chain: DiggerProofArchive → RefineryProofArchive → MintProofArchive
//   - Verification: Query archives to validate merkle path
//
// Data Flow:
//   - Digger stores: JouleTorqUnit details (3.6M units)
//   - Refinery stores: TokenTorqIngot details (1000 ingots) + metadata
//   - Mint stores: Phase3RoboTorqUnit (merkle root only)
//   - DistoDam receives: Phase3RoboTorqUnit for ledger
//
// Value:
//   - 1 RT = 1000 ingots × 3600 units = 3,600,000 tokens
//   - Represents 3.6M joules of verified robotic labor
//
// Metadata Reconstruction:
//   - ContractIDs, DiggerIDs, RefineryIDs, TotalJoules, TotalRoboStake
//   - All reconstructed by querying proof archives (not stored here)
//   - Query flow: RT unit → Mint archive (1000 ingot hashes) → Refinery archive (ingot details)
//
// NFC Tag Compatibility:
//   - NTAG215 has 540 bytes usable
//   - Phase3RoboTorqUnit JSON: ~150 bytes
//   - Signature + printer key: ~128 bytes
//   - Total: ~280 bytes (leaves 260 bytes for future use)
//
// Proof Verification:
//   - User queries: "Does token X exist in RT unit Y?"
//   - Digger returns: JTU hash for token X
//   - Refinery returns: Ingot hash containing token X
//   - Mint returns: Merkle path from ingot hash → root
//   - User verifies: Path matches Phase3RoboTorqUnit.MerkleRoot
type Phase3RoboTorqUnit struct {
	// UnitID uniquely identifies this 1 RT unit
	// Format: RT-{timestamp}
	UnitID string `json:"unit_id"`

	// MerkleRoot is the Level 2 merkle root (64-char hex SHA256)
	// Built from 1000 ingot hashes (each representing 3600 units)
	// This is the ONLY cryptographic proof stored on-chain
	MerkleRoot string `json:"merkle_root"`

	// MintedAt is when this RT unit was created
	MintedAt time.Time `json:"minted_at"`

	// Signature is the SPHINCS+ signature over (UnitID + MerkleRoot + MintedAt)
	// SPHINCS+ chosen for archival security (stateless, paranoid, quantum-proof)
	// ~49KB signature size (large but acceptable for long-term storage)
	Signature string `json:"signature,omitempty"`

	// PublicKey is the Mint's SPHINCS+ public key for verification
	// Allows anyone to verify this RT unit's authenticity
	PublicKey string `json:"public_key,omitempty"`
}

// NewPhase3RoboTorqUnit creates a new Phase 3 RT unit from merkle root
//
// Parameters:
//   - merkleRoot: Level 2 merkle root (64-char hex from 1000 ingot hashes)
//
// Returns:
//   - Phase3RoboTorqUnit ready for DistoDam publishing
//   - Error if validation fails
//
// Note: Metadata (contracts, diggers, refineries, joules, robo stake) is NOT stored.
// It can be reconstructed by querying proof archives when needed.
func NewPhase3RoboTorqUnit(merkleRoot string) (*Phase3RoboTorqUnit, error) {
	// Validate merkle root format (64-char hex SHA256)
	if len(merkleRoot) != 64 {
		return nil, fmt.Errorf("invalid merkle_root length: got %d, expected 64", len(merkleRoot))
	}

	// Verify it's valid hex
	for _, c := range merkleRoot {
		if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')) {
			return nil, fmt.Errorf("invalid merkle_root: must be hex string, got invalid char: %c", c)
		}
	}

	unit := &Phase3RoboTorqUnit{
		UnitID:     generateUnitID(),
		MerkleRoot: merkleRoot,
		MintedAt:   time.Now().UTC(),
	}

	return unit, nil
}

// Validate checks that the unit has valid data
func (u *Phase3RoboTorqUnit) Validate() error {
	if u.UnitID == "" {
		return fmt.Errorf("unit_id is required")
	}

	if len(u.MerkleRoot) != 64 {
		return fmt.Errorf("merkle_root must be 64-char hex, got %d", len(u.MerkleRoot))
	}

	// Verify it's valid hex
	for _, c := range u.MerkleRoot {
		if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')) {
			return fmt.Errorf("merkle_root must be hex string")
		}
	}

	if u.MintedAt.IsZero() {
		return fmt.Errorf("minted_at timestamp is required")
	}

	return nil
}

// ToJSON serializes the unit for NATS publishing
func (u *Phase3RoboTorqUnit) ToJSON() ([]byte, error) {
	return json.Marshal(u)
}

// generateUnitID creates a unique ID for the RT unit
func generateUnitID() string {
	// Format: RT-{timestamp}
	return fmt.Sprintf("RT-%s", time.Now().UTC().Format("20060102-150405.000000"))
}

// SizeBytes returns approximate JSON size in bytes
func (u *Phase3RoboTorqUnit) SizeBytes() int {
	data, _ := u.ToJSON()
	return len(data)
}
