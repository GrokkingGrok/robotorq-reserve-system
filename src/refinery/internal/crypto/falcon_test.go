//go:build cgo
// +build cgo

package crypto

import (
	"encoding/hex"
	"testing"

	"github.com/open-quantum-safe/liboqs-go/oqs"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// NOTE: These tests require liboqs C library (pkg-config)
// They run in Docker but may be skipped locally if liboqs not installed

// ============================================================================
// FalconSigner Tests (Phase 5)
// ============================================================================

func TestFalconSigner_NewFalconSigner(t *testing.T) {
	// Test that signer initializes with valid keypair
	signer, err := NewFalconSigner()
	require.NoError(t, err, "NewFalconSigner should succeed")
	require.NotNil(t, signer, "Signer should not be nil")
	defer signer.Clean()

	// Verify secret key was exported and stored
	assert.NotNil(t, signer.secretKey, "Secret key should be stored")
	assert.NotEmpty(t, signer.secretKey, "Secret key should not be empty")

	// Verify public key was generated
	assert.NotNil(t, signer.publicKey, "Public key should be stored")
	assert.NotEmpty(t, signer.publicKey, "Public key should not be empty")

	// Falcon-1024 key sizes from liboqs (expanded format, not compact)
	assert.Equal(t, 1793, len(signer.publicKey), "Falcon-1024 public key from liboqs (expanded)")
	assert.Equal(t, 2305, len(signer.secretKey), "Falcon-1024 secret key from liboqs (expanded)")
}

func TestFalconSigner_SignPhase2Ingot_SingleSignature(t *testing.T) {
	// Test that signing produces valid signature
	signer, err := NewFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Sign a Phase2Ingot message
	ingotID := "test-ingot-001"
	branchHash := "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
	hashCount := 3600
	timestamp := "2025-11-16T12:00:00Z"

	signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err, "Signing should succeed")

	// Verify signature is hex-encoded
	assert.NotEmpty(t, signatureHex, "Signature should not be empty")
	_, err = hex.DecodeString(signatureHex)
	assert.NoError(t, err, "Signature should be valid hex")

	// Verify public key is hex-encoded
	assert.NotEmpty(t, publicKeyHex, "Public key should not be empty")
	_, err = hex.DecodeString(publicKeyHex)
	assert.NoError(t, err, "Public key should be valid hex")

	// Falcon-1024 signature is variable length (around 1200-1300 bytes with expanded keys)
	sigBytes, _ := hex.DecodeString(signatureHex)
	assert.GreaterOrEqual(t, len(sigBytes), 1100, "Signature should be at least 1100 bytes")
	assert.LessOrEqual(t, len(sigBytes), 1400, "Signature should be at most 1400 bytes")
}

func TestFalconSigner_SignPhase2Ingot_MultipleSignatures(t *testing.T) {
	// REGRESSION TEST: Verify we can sign multiple messages with same signer
	// This was the bug we fixed - re-initializing signature object with secret key
	signer, err := NewFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Sign 10 different messages (simulating 10 ingots)
	signatures := make([]string, 10)
	for i := 0; i < 10; i++ {
		ingotID := "test-ingot-" + string(rune('0'+i))
		branchHash := "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
		hashCount := 3600
		timestamp := "2025-11-16T12:00:00Z"

		signatureHex, _, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
		require.NoError(t, err, "Signing ingot %d should succeed", i+1)
		signatures[i] = signatureHex
	}

	// Verify all signatures are unique (different ingot IDs → different signatures)
	uniqueSigs := make(map[string]bool)
	for _, sig := range signatures {
		uniqueSigs[sig] = true
	}
	assert.Equal(t, 10, len(uniqueSigs), "All 10 signatures should be unique")
}

func TestFalconSigner_Clean_ErasesSecretKey(t *testing.T) {
	// Test that Clean() securely erases the secret key
	signer, err := NewFalconSigner()
	require.NoError(t, err)

	// Verify secret key exists before Clean()
	assert.NotNil(t, signer.secretKey)
	assert.NotEmpty(t, signer.secretKey)
	secretKeyLen := len(signer.secretKey)

	// Call Clean()
	signer.Clean()

	// Verify secret key is nil after Clean()
	assert.Nil(t, signer.secretKey, "Secret key should be nil after Clean()")

	// Note: We can't verify MemCleanse() actually zeroed the memory,
	// but we trust oqs.MemCleanse() does the right thing
	_ = secretKeyLen
}

func TestFalconSigner_SignAndVerify_RoundTrip(t *testing.T) {
	// Test that signed messages can be verified successfully
	signer, err := NewFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Sign a message
	ingotID := "roundtrip-ingot-001"
	branchHash := "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
	hashCount := 3600
	timestamp := "2025-11-16T15:30:00Z"

	signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err)

	// Verify the signature using oqs.Signature directly
	sig := oqs.Signature{}
	defer sig.Clean()

	err = sig.Init("Falcon-1024", nil)
	require.NoError(t, err)

	// Reconstruct the message
	message := ingotID + "|" + branchHash + "|3600|" + timestamp

	// Decode hex
	signatureBytes, err := hex.DecodeString(signatureHex)
	require.NoError(t, err)
	publicKeyBytes, err := hex.DecodeString(publicKeyHex)
	require.NoError(t, err)

	// Verify signature
	isValid, err := sig.Verify([]byte(message), signatureBytes, publicKeyBytes)
	require.NoError(t, err, "Verification should not error")
	assert.True(t, isValid, "Signature should be valid")
}

// ============================================================================
// FalconVerifier Tests (Phase 5 - Hash Batch Verification from Digger)
// ============================================================================

func TestFalconVerifier_ValidSignature(t *testing.T) {
	// Test valid signature verification from hash batch
	verifier := NewFalconVerifier()

	// Create a signer to generate a real signature
	signer, err := NewFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Prepare hash batch data
	contractID := "test-contract-001"
	diggerID := "test-digger-001"
	milestoneIndex := uint32(0)
	joules := 0.0
	roboStake := 0.0
	unitHashes := []string{
		"hash1aabbccdd",
		"hash2eeffgghh",
		"hash3iijjkkll",
	}
	timestamp := "2025-11-16T12:00:00Z"

	// Reconstruct message exactly as Digger would (and as verifier expects)
	hashesJoined := "hash1aabbccdd,hash2eeffgghh,hash3iijjkkll"
	message := contractID + "|" + diggerID + "|0|0.00|0.00000000|" + hashesJoined + "|" + timestamp

	// Sign with Falcon
	sig := oqs.Signature{}
	defer sig.Clean()
	sig.Init("Falcon-1024", nil)
	publicKey, _ := sig.GenerateKeyPair()
	signature, _ := sig.Sign([]byte(message))

	signatureHex := hex.EncodeToString(signature)
	publicKeyHex := hex.EncodeToString(publicKey)

	// Verify using FalconVerifier
	err = verifier.VerifyHashBatch(
		contractID,
		diggerID,
		milestoneIndex,
		joules,
		roboStake,
		unitHashes,
		timestamp,
		signatureHex,
		publicKeyHex,
	)

	assert.NoError(t, err, "Valid signature should verify successfully")
}

func TestFalconVerifier_InvalidSignature(t *testing.T) {
	// Test that invalid signature is rejected
	verifier := NewFalconVerifier()

	// Generate valid keypair
	sig := oqs.Signature{}
	defer sig.Clean()
	sig.Init("Falcon-1024", nil)
	publicKey, _ := sig.GenerateKeyPair()

	// Create random invalid signature (wrong bytes)
	invalidSignature := make([]byte, 666)
	for i := range invalidSignature {
		invalidSignature[i] = byte(i % 256)
	}

	signatureHex := hex.EncodeToString(invalidSignature)
	publicKeyHex := hex.EncodeToString(publicKey)

	// Try to verify
	err := verifier.VerifyHashBatch(
		"contract",
		"digger",
		0,
		0.0,
		0.0,
		[]string{"hash1"},
		"2025-11-16T12:00:00Z",
		signatureHex,
		publicKeyHex,
	)

	assert.Error(t, err, "Invalid signature should be rejected")
	assert.Contains(t, err.Error(), "invalid", "Error should mention invalid signature")
}

func TestFalconVerifier_TamperedMessage(t *testing.T) {
	// Test that tampering with message is detected
	verifier := NewFalconVerifier()

	// Create valid signature for original message
	sig := oqs.Signature{}
	defer sig.Clean()
	sig.Init("Falcon-1024", nil)
	publicKey, _ := sig.GenerateKeyPair()

	originalMessage := "contract-001|digger-001|0|0.00|0.00000000|hash1,hash2|2025-11-16T12:00:00Z"
	signature, _ := sig.Sign([]byte(originalMessage))

	signatureHex := hex.EncodeToString(signature)
	publicKeyHex := hex.EncodeToString(publicKey)

	// Try to verify with TAMPERED hashes (different from what was signed)
	err := verifier.VerifyHashBatch(
		"contract-001",
		"digger-001",
		0,
		0.0,
		0.0,
		[]string{"TAMPERED_hash1", "TAMPERED_hash2"}, // ← Changed!
		"2025-11-16T12:00:00Z",
		signatureHex,
		publicKeyHex,
	)

	assert.Error(t, err, "Tampered message should fail verification")
	assert.Contains(t, err.Error(), "invalid", "Error should indicate signature mismatch")
}

func TestFalconVerifier_WrongPublicKey(t *testing.T) {
	// Test that using wrong public key fails verification
	verifier := NewFalconVerifier()

	// Generate signature with one keypair
	sig1 := oqs.Signature{}
	defer sig1.Clean()
	sig1.Init("Falcon-1024", nil)
	sig1.GenerateKeyPair()

	message := "contract|digger|0|0.00|0.00000000|hash1|2025-11-16T12:00:00Z"
	signature, _ := sig1.Sign([]byte(message))

	// Generate DIFFERENT public key
	sig2 := oqs.Signature{}
	defer sig2.Clean()
	sig2.Init("Falcon-1024", nil)
	wrongPublicKey, _ := sig2.GenerateKeyPair()

	signatureHex := hex.EncodeToString(signature)
	publicKeyHex := hex.EncodeToString(wrongPublicKey) // ← Wrong key!

	// Try to verify
	err := verifier.VerifyHashBatch(
		"contract",
		"digger",
		0,
		0.0,
		0.0,
		[]string{"hash1"},
		"2025-11-16T12:00:00Z",
		signatureHex,
		publicKeyHex,
	)

	assert.Error(t, err, "Wrong public key should fail verification")
	assert.Contains(t, err.Error(), "invalid", "Error should indicate signature failure")
}

func TestFalconVerifier_InvalidHex(t *testing.T) {
	// Test hex validation (doesn't require liboqs signature verification)
	verifier := NewFalconVerifier()

	err := verifier.VerifyHashBatch(
		"contract",
		"digger",
		0,
		0.0,
		0.0,
		[]string{"hash1"},
		"2025-11-16T12:00:00Z",
		"NOT_VALID_HEX", // Invalid hex
		"",
	)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid signature hex")
}
