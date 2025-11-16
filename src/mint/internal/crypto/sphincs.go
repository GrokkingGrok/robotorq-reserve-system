package crypto

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
)

// SPHINCSPlusSigner handles SPHINCS+ signature generation for Phase3RoboTorqUnits
//
// NOTE: Phase 4 Crypto Implementation Status
// ===========================================
// SPHINCS+ is for ARCHIVAL signatures (long-term security, slower is okay)
//
// Why SPHINCS+ instead of Falcon:
// - Paranoid security: Stateless hash-based signatures
// - No secret key compromise: Even if key leaks, old signatures stay valid
// - Quantum-proof: Based on hash functions, not lattices
// - Slower: ~50ms signing vs Falcon's 0.5ms (acceptable for 1 RT/min creation)
//
// TODO for Production:
// 1. Use liboqs-go (github.com/open-quantum-safe/liboqs-go) which has SPHINCS+
// 2. OR use CIRCL if they add SPHINCS+ support
// 3. OR use CGO bindings to pqclean's SPHINCS+ implementation
//
// For now: Mint creates Phase3Units but SKIPS signing (DEVELOPMENT ONLY!)
type SPHINCSPlusSigner struct {
}

// NewSPHINCSPlusSigner creates a new SPHINCS+ signer
func NewSPHINCSPlusSigner() *SPHINCSPlusSigner {
	// TODO(phase-4-production): Implement actual SPHINCS+ signing
	// Options same as Falcon: liboqs-go, CIRCL (future), or CGO bindings

	return &SPHINCSPlusSigner{}
}

// SignPhase3Unit signs a Phase3RoboTorqUnit merkle root with SPHINCS+
//
// # Arguments
//   - unitID: The RT unit ID (e.g., "RT-20251116-120000.000000")
//   - merkleRoot: The Level 2 merkle root (64-char hex)
//   - mintedAt: Timestamp when unit was created (RFC3339)
//
// # Returns
//   - signatureHex: Hex-encoded SPHINCS+ signature
//   - publicKeyHex: Hex-encoded SPHINCS+ public key
//   - error if signing fails
//
// ⚠️  CURRENT STATUS: Returns placeholder signature - DEVELOPMENT ONLY!
func (s *SPHINCSPlusSigner) SignPhase3Unit(
	unitID string,
	merkleRoot string,
	mintedAt string,
) (signatureHex string, publicKeyHex string, error error) {
	// Hash the unit data for deterministic signing
	unitHash := hashUnitForSigning(unitID, merkleRoot, mintedAt)

	// Validate hex format at least
	if len(merkleRoot) != 64 {
		return "", "", fmt.Errorf("invalid merkle root length: %d", len(merkleRoot))
	}

	// TODO(phase-4-production): IMPLEMENT ACTUAL SPHINCS+ SIGNING
	// For now, return placeholder (allows pipeline to work)

	// ⚠️  WARNING: These are FAKE signatures! NOT for production!
	// Production must use real SPHINCS+ signing for archival security.

	placeholderSignature := hex.EncodeToString(unitHash[:]) // Just the hash
	placeholderPublicKey := hex.EncodeToString([]byte("PLACEHOLDER_SPHINCS_PUBLIC_KEY_32_BYTES"))

	return placeholderSignature, placeholderPublicKey, nil
}

// VerifyPhase3Unit verifies a SPHINCS+ signature on a Phase3RoboTorqUnit
//
// # Arguments
//   - unitID: The RT unit ID
//   - merkleRoot: The Level 2 merkle root
//   - mintedAt: Timestamp (RFC3339)
//   - signatureHex: Hex-encoded SPHINCS+ signature
//   - publicKeyHex: Hex-encoded SPHINCS+ public key
//
// # Returns
//   - error if signature invalid, nil if valid
//
// ⚠️  CURRENT STATUS: Returns nil (accepts all signatures) - DEVELOPMENT ONLY!
func (s *SPHINCSPlusSigner) VerifyPhase3Unit(
	unitID string,
	merkleRoot string,
	mintedAt string,
	signatureHex string,
	publicKeyHex string,
) error {
	// Validate hex format
	_, err := hex.DecodeString(signatureHex)
	if err != nil {
		return fmt.Errorf("invalid signature hex: %w", err)
	}

	_, err = hex.DecodeString(publicKeyHex)
	if err != nil {
		return fmt.Errorf("invalid public key hex: %w", err)
	}

	// TODO(phase-4-production): IMPLEMENT ACTUAL SPHINCS+ VERIFICATION
	// For now, just validate hex encoding (development/testing phase)

	// ⚠️  WARNING: This accepts ALL signatures! NOT for production!

	return nil // PLACEHOLDER - accepts all signatures
}

// hashUnitForSigning creates a deterministic SHA256 hash of unit data
func hashUnitForSigning(unitID string, merkleRoot string, mintedAt string) [32]byte {
	hasher := sha256.New()

	// Hash in deterministic order
	hasher.Write([]byte(unitID))
	hasher.Write([]byte(merkleRoot))
	hasher.Write([]byte(mintedAt))

	var result [32]byte
	copy(result[:], hasher.Sum(nil))
	return result
}
