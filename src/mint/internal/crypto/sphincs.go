//go:build cgo

package crypto

import (
	"encoding/hex"
	"fmt"
	"log/slog"

	"github.com/open-quantum-safe/liboqs-go/oqs"
)

// SPHINCSPlusSigner handles SPHINCS+ signature generation for Phase3RoboTorqUnits
//
// Phase 5 Implementation: ARCHIVAL Signatures for Phase3 Units
// ==============================================================
// SPHINCS+ provides long-term security (no trapdoors, hash-based)
// for archival storage of RoboTorq units. Unlike Falcon (optimized for
// speed), SPHINCS+ signatures are larger but mathematically provable
// security.
//
// Uses liboqs-go with SPHINCS+-SHA2-128f-simple variant:
// - SHA2-128f: Uses SHA-256 (well-understood hash function)
// - simple: Simpler variant (easier to audit)
// - f: Fast variant (faster signing, larger signatures)
// - Security: 128-bit (matches Falcon-1024)
//
// Why SPHINCS+ instead of Falcon:
// - Paranoid security: Stateless hash-based signatures
// - No secret key compromise: Even if key leaks, old signatures stay valid
// - Quantum-proof: Based on hash functions, not lattices
// - Slower: ~50ms signing vs Falcon's 0.5ms (acceptable for 1 RT/min creation)
type SPHINCSPlusSigner struct {
	logger     *slog.Logger
	privateKey []byte
	publicKey  []byte
}

// NewSPHINCSPlusSigner creates a new SPHINCS+ signer with generated keypair
func NewSPHINCSPlusSigner(logger *slog.Logger) (*SPHINCSPlusSigner, error) {
	// Initialize SPHINCS+ signature scheme
	sig := oqs.Signature{}
	defer sig.Clean()

	if err := sig.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		return nil, fmt.Errorf("failed to initialize SPHINCS+: %w", err)
	}

	// Generate keypair
	publicKey, err := sig.GenerateKeyPair()
	if err != nil {
		return nil, fmt.Errorf("failed to generate SPHINCS+ keypair: %w", err)
	}

	// Export secret key and COPY it (liboqs may modify the original)
	exportedKey := sig.ExportSecretKey()
	privateKey := make([]byte, len(exportedKey))
	copy(privateKey, exportedKey)

	logger.Info("SPHINCS+ keypair generated",
		"algorithm", "SPHINCS+-SHA2-128f-simple",
		"public_key_size", len(publicKey),
		"private_key_size", len(privateKey))

	return &SPHINCSPlusSigner{
		logger:     logger,
		privateKey: privateKey,
		publicKey:  publicKey,
	}, nil
}

// SignPhase3Unit signs a Phase3RoboTorqUnit with SPHINCS+
//
// # Arguments
//   - unitID: The unit ID (e.g., "unit-20251116-001")
//   - merkleRoot: The Level2 merkle root hash (64-char hex)
//   - mintedAt: ISO8601 timestamp when unit was minted
//
// # Returns
//   - signatureHex: Hex-encoded SPHINCS+ signature (~17KB for -128s variant)
//   - publicKeyHex: Hex-encoded SPHINCS+ public key (~32 bytes)
//   - error if signing failed
func (s *SPHINCSPlusSigner) SignPhase3Unit(
	unitID string,
	merkleRoot string,
	mintedAt string,
) (signatureHex string, publicKeyHex string, error error) {
	// Initialize signer
	sig := oqs.Signature{}
	defer sig.Clean()

	// CRITICAL: Copy private key before passing to Init()
	// liboqs-go stores a reference and may zero it on Clean()
	privateKeyCopy := make([]byte, len(s.privateKey))
	copy(privateKeyCopy, s.privateKey)

	if err := sig.Init("SPHINCS+-SHA2-128f-simple", privateKeyCopy); err != nil {
		return "", "", fmt.Errorf("failed to initialize SPHINCS+: %w", err)
	}

	// Create canonical message
	message := createSigningMessage(unitID, merkleRoot, mintedAt)

	// Sign message
	signature, err := sig.Sign([]byte(message))
	if err != nil {
		return "", "", fmt.Errorf("failed to sign Phase3 unit: %w", err)
	}

	signatureHex = hex.EncodeToString(signature)
	publicKeyHex = hex.EncodeToString(s.publicKey)

	s.logger.Debug("SPHINCS+ signature generated",
		"unit_id", unitID,
		"signature_size", len(signature),
		"public_key_size", len(s.publicKey))

	return signatureHex, publicKeyHex, nil
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
func (s *SPHINCSPlusSigner) VerifyPhase3Unit(
	unitID string,
	merkleRoot string,
	mintedAt string,
	signatureHex string,
	publicKeyHex string,
) error {
	// Decode public key
	publicKey, err := hex.DecodeString(publicKeyHex)
	if err != nil {
		return fmt.Errorf("invalid public key hex: %w", err)
	}

	// Decode signature
	signature, err := hex.DecodeString(signatureHex)
	if err != nil {
		return fmt.Errorf("invalid signature hex: %w", err)
	}

	// Initialize verifier
	sig := oqs.Signature{}
	defer sig.Clean()

	if err := sig.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		return fmt.Errorf("failed to initialize SPHINCS+: %w", err)
	}

	// Create canonical message
	message := createSigningMessage(unitID, merkleRoot, mintedAt)

	// Verify signature
	isValid, err := sig.Verify([]byte(message), signature, publicKey)
	if err != nil {
		return fmt.Errorf("signature verification failed: %w", err)
	}

	if !isValid {
		return fmt.Errorf("invalid SPHINCS+ signature for unit=%s", unitID)
	}

	return nil
}

// GetPublicKey returns the hex-encoded public key
func (s *SPHINCSPlusSigner) GetPublicKey() string {
	return hex.EncodeToString(s.publicKey)
}

// createSigningMessage creates the canonical message to sign/verify
// Must match format: "{unitID}|{merkleRoot}|{mintedAt}"
func createSigningMessage(unitID, merkleRoot, mintedAt string) string {
	return fmt.Sprintf("%s|%s|%s", unitID, merkleRoot, mintedAt)
}

// hashUnitForSigning - DEPRECATED, kept for backward compatibility
// Use createSigningMessage instead
func hashUnitForSigning(unitID string, merkleRoot string, mintedAt string) [32]byte {
	// This was the old placeholder implementation
	// Keeping for any tests that might reference it
	message := createSigningMessage(unitID, merkleRoot, mintedAt)
	var result [32]byte
	copy(result[:], []byte(message))
	return result
}
