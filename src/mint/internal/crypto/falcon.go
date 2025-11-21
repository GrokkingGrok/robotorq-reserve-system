//go:build cgo

package crypto

import (
	"encoding/hex"
	"fmt"

	"github.com/open-quantum-safe/liboqs-go/oqs"
)

// FalconVerifier handles Falcon-1024 signature verification for Phase2Ingots
//
// Phase 5 Implementation: REAL Falcon-1024 Verification in Mint
// ==============================================================
// Uses liboqs-go (github.com/open-quantum-safe/liboqs-go) for NIST-approved
// post-quantum signature verification.
//
// Verifies signatures created by Refinery on Phase2Ingots (merkle branch hashes)
type FalconVerifier struct {
}

// NewFalconVerifier creates a new Falcon-1024 verifier
func NewFalconVerifier() *FalconVerifier {
	return &FalconVerifier{}
}

// VerifyPhase2Ingot verifies a Falcon-1024 signature on a Phase2Ingot
//
// # Arguments
//   - ingotID: The ingot ID (e.g., "ingot-20251116-001")
//   - branchHash: The merkle root hash (64-char hex)
//   - hashCount: Number of hashes in merkle tree (should be 3600)
//   - timestamp: ISO8601 timestamp when ingot was assembled
//   - signatureHex: Hex-encoded Falcon-1024 signature from Refinery
//   - publicKeyHex: Hex-encoded Falcon-1024 public key from Refinery
//
// # Returns
//   - error if signature invalid, nil if valid
func (fv *FalconVerifier) VerifyPhase2Ingot(
	ingotID string,
	branchHash string,
	hashCount int,
	timestamp string,
	signatureHex string,
	publicKeyHex string,
) error {
	// 1. Decode public key
	publicKey, err := hex.DecodeString(publicKeyHex)
	if err != nil {
		return fmt.Errorf("invalid public key hex: %w", err)
	}

	// 2. Decode signature
	signature, err := hex.DecodeString(signatureHex)
	if err != nil {
		return fmt.Errorf("invalid signature hex: %w", err)
	}

	// 3. Reconstruct the message that was signed by Refinery
	// Must match EXACTLY what Refinery signs:
	// format!("{ingot_id}|{branch_hash}|{hash_count}|{timestamp}")
	message := fmt.Sprintf("%s|%s|%d|%s",
		ingotID,
		branchHash,
		hashCount,
		timestamp)

	// 4. Initialize Falcon-1024 verifier
	sig := oqs.Signature{}
	defer sig.Clean()

	if err := sig.Init("Falcon-1024", nil); err != nil {
		return fmt.Errorf("failed to initialize Falcon-1024: %w", err)
	}

	// 5. Verify the signature
	isValid, err := sig.Verify([]byte(message), signature, publicKey)
	if err != nil {
		return fmt.Errorf("signature verification failed: %w", err)
	}

	if !isValid {
		return fmt.Errorf("invalid Falcon-1024 signature for ingot=%s", ingotID)
	}

	return nil
}
