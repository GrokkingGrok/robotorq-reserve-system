package mint

import (
	"fmt"
	"sync"
)

// SignatureRecord stores a SPHINCS+ signature for a Phase3RoboTorqUnit
type SignatureRecord struct {
	UnitID     string `json:"unit_id"`
	Signature  string `json:"signature"`   // Hex-encoded SPHINCS+ signature (~17KB)
	PublicKey  string `json:"public_key"`  // Hex-encoded SPHINCS+ public key (~32 bytes)
	MerkleRoot string `json:"merkle_root"` // For verification
	MintedAt   string `json:"minted_at"`   // ISO8601 timestamp
	SignedAt   string `json:"signed_at"`   // When signature was generated
}

// SignatureArchive stores SPHINCS+ signatures separately from Phase3 units
//
// Architecture:
//   - Phase3RoboTorqUnit JSON: 243 bytes (fits NTAG216 NFC tag)
//   - SignatureArchive: Full SPHINCS+ signatures for verification
//   - API endpoint: GET /verify/signature/{unit_id} returns SignatureRecord
//
// Why separate storage:
//   - SPHINCS+ signatures are ~17KB (too large for NFC tags)
//   - NFC tags only need merkle proof (243 bytes)
//   - Full signatures stored in Mint for API-based verification
//   - DistoDam ledger receives units + signatures for permanent archival
type SignatureArchive struct {
	signatures map[string]*SignatureRecord
	mu         sync.RWMutex
}

// NewSignatureArchive creates a new signature archive
func NewSignatureArchive() *SignatureArchive {
	return &SignatureArchive{
		signatures: make(map[string]*SignatureRecord),
	}
}

// Store saves a signature record for a unit
func (sa *SignatureArchive) Store(record *SignatureRecord) error {
	if record.UnitID == "" {
		return fmt.Errorf("unit_id is required")
	}
	if record.Signature == "" {
		return fmt.Errorf("signature is required")
	}
	if record.PublicKey == "" {
		return fmt.Errorf("public_key is required")
	}

	sa.mu.Lock()
	defer sa.mu.Unlock()

	sa.signatures[record.UnitID] = record
	return nil
}

// Get retrieves a signature record by unit ID
func (sa *SignatureArchive) Get(unitID string) (*SignatureRecord, error) {
	sa.mu.RLock()
	defer sa.mu.RUnlock()

	record, exists := sa.signatures[unitID]
	if !exists {
		return nil, fmt.Errorf("signature not found for unit_id=%s", unitID)
	}

	return record, nil
}

// Size returns the number of signatures stored
func (sa *SignatureArchive) Size() int {
	sa.mu.RLock()
	defer sa.mu.RUnlock()
	return len(sa.signatures)
}

// Has checks if a signature exists for a unit
func (sa *SignatureArchive) Has(unitID string) bool {
	sa.mu.RLock()
	defer sa.mu.RUnlock()
	_, exists := sa.signatures[unitID]
	return exists
}
