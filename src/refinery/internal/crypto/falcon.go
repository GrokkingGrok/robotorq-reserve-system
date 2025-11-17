package crypto

import (
	"encoding/hex"
	"fmt"
	"strings"

	"github.com/open-quantum-safe/liboqs-go/oqs"
)

// FalconVerifier handles Falcon-1024 signature verification
//
// Phase 5 Implementation: REAL Falcon-1024 Verification
// =======================================================
// Uses liboqs-go (github.com/open-quantum-safe/liboqs-go) for NIST-approved
// post-quantum signature verification.
//
// Verifies signatures created by Digger (Rust + pqcrypto-falcon)
type FalconVerifier struct {
}

// NewFalconVerifier creates a new Falcon-1024 verifier
func NewFalconVerifier() *FalconVerifier {
	return &FalconVerifier{}
}

// VerifyHashBatch verifies a Falcon-1024 signature on a hash batch from Digger
//
// # Arguments
//   - contractID: The contract ID
//   - diggerID: The digger ID
//   - milestoneIndex: Milestone index (0 for simplified batches)
//   - joules: Total joules (0.0 for hash batches)
//   - roboStake: Total robo stake (0.0 for hash batches)
//   - unitHashes: Array of JTU hashes
//   - timestamp: ISO8601 timestamp
//   - signatureHex: Hex-encoded Falcon-1024 signature
//   - publicKeyHex: Hex-encoded Falcon-1024 public key
//
// # Returns
//   - error if signature invalid, nil if valid
func (fv *FalconVerifier) VerifyHashBatch(
	contractID string,
	diggerID string,
	milestoneIndex uint32,
	joules float64,
	roboStake float64,
	unitHashes []string,
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

	// 3. Reconstruct the message that was signed
	// Must match EXACTLY what Digger signed in Rust:
	// format!("{contract_id}|{digger_id}|{milestone_index}|{joules}|{robo_stake}|{hashes_joined}|{timestamp}")
	hashesJoined := strings.Join(unitHashes, ",")
	message := fmt.Sprintf("%s|%s|%d|%.2f|%.8f|%s|%s",
		contractID,
		diggerID,
		milestoneIndex,
		joules,
		roboStake,
		hashesJoined,
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
		return fmt.Errorf("invalid Falcon-1024 signature for contract=%s digger=%s", contractID, diggerID)
	}

	return nil
}
