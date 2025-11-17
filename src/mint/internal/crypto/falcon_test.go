//go:build cgo
// +build cgo

package crypto

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestFalconVerifier_VerifyPhase2Ingot_ValidSignature tests valid signature verification
func TestFalconVerifier_VerifyPhase2Ingot_ValidSignature(t *testing.T) {
	// We need to generate a real signature using the same library
	// This simulates what Refinery does
	signer := createTestSigner(t)
	defer signer.Clean()

	// Test data
	ingotID := "test-ingot-001"
	branchHash := "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
	hashCount := 3600
	timestamp := "2025-11-16T12:00:00Z"

	// Sign with the test signer
	signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err, "Test signer should create valid signature")

	// Now verify it
	verifier := NewFalconVerifier()
	err = verifier.VerifyPhase2Ingot(ingotID, branchHash, hashCount, timestamp, signatureHex, publicKeyHex)

	assert.NoError(t, err, "Valid signature should verify successfully")
}

// TestFalconVerifier_VerifyPhase2Ingot_InvalidSignature tests rejection of invalid signatures
func TestFalconVerifier_VerifyPhase2Ingot_InvalidSignature(t *testing.T) {
	signer := createTestSigner(t)
	defer signer.Clean()

	ingotID := "test-ingot-002"
	branchHash := "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
	hashCount := 3600
	timestamp := "2025-11-16T12:00:00Z"

	// Get real signature
	signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err)

	// Corrupt the signature (flip first byte)
	corruptSignature := "ff" + signatureHex[2:]

	// Verify should fail
	verifier := NewFalconVerifier()
	err = verifier.VerifyPhase2Ingot(ingotID, branchHash, hashCount, timestamp, corruptSignature, publicKeyHex)

	assert.Error(t, err, "Corrupted signature should fail verification")
	assert.Contains(t, err.Error(), "invalid Falcon-1024 signature", "Error should indicate invalid signature")
}

// TestFalconVerifier_VerifyPhase2Ingot_TamperedMessage tests detection of message tampering
func TestFalconVerifier_VerifyPhase2Ingot_TamperedMessage(t *testing.T) {
	signer := createTestSigner(t)
	defer signer.Clean()

	// Original message
	ingotID := "test-ingot-003"
	branchHash := "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
	hashCount := 3600
	timestamp := "2025-11-16T12:00:00Z"

	signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err)

	// Tamper with the message (different branch hash)
	tamperedBranchHash := "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"

	// Verification should fail
	verifier := NewFalconVerifier()
	err = verifier.VerifyPhase2Ingot(ingotID, tamperedBranchHash, hashCount, timestamp, signatureHex, publicKeyHex)

	assert.Error(t, err, "Tampered message should fail verification")
	assert.Contains(t, err.Error(), "invalid Falcon-1024 signature", "Error should indicate signature mismatch")
}

// TestFalconVerifier_VerifyPhase2Ingot_WrongPublicKey tests rejection with wrong public key
func TestFalconVerifier_VerifyPhase2Ingot_WrongPublicKey(t *testing.T) {
	signer1 := createTestSigner(t)
	defer signer1.Clean()

	signer2 := createTestSigner(t)
	defer signer2.Clean()

	ingotID := "test-ingot-004"
	branchHash := "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
	hashCount := 3600
	timestamp := "2025-11-16T12:00:00Z"

	// Sign with signer1
	signatureHex, _, err := signer1.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err)

	// Get public key from signer2 (different key!)
	_, wrongPublicKeyHex, err := signer2.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
	require.NoError(t, err)

	// Verify with wrong public key should fail
	verifier := NewFalconVerifier()
	err = verifier.VerifyPhase2Ingot(ingotID, branchHash, hashCount, timestamp, signatureHex, wrongPublicKeyHex)

	assert.Error(t, err, "Wrong public key should fail verification")
	assert.Contains(t, err.Error(), "invalid Falcon-1024 signature", "Error should indicate verification failure")
}

// TestFalconVerifier_VerifyPhase2Ingot_InvalidHex tests hex validation
func TestFalconVerifier_VerifyPhase2Ingot_InvalidHex(t *testing.T) {
	verifier := NewFalconVerifier()

	tests := []struct {
		name         string
		signatureHex string
		publicKeyHex string
		expectError  string
	}{
		{
			name:         "invalid signature hex",
			signatureHex: "NOT_HEX_DATA",
			publicKeyHex: "abcdef1234567890",
			expectError:  "invalid signature hex",
		},
		{
			name:         "invalid public key hex",
			signatureHex: "abcdef1234567890",
			publicKeyHex: "NOT_HEX_DATA",
			expectError:  "invalid public key hex",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := verifier.VerifyPhase2Ingot(
				"test-ingot",
				"hash",
				3600,
				"2025-11-16T12:00:00Z",
				tt.signatureHex,
				tt.publicKeyHex,
			)

			assert.Error(t, err)
			assert.Contains(t, err.Error(), tt.expectError)
		})
	}
}

// TestFalconVerifier_MultipleVerifications tests that verifier can be reused
func TestFalconVerifier_MultipleVerifications(t *testing.T) {
	signer := createTestSigner(t)
	defer signer.Clean()

	verifier := NewFalconVerifier()

	// Verify multiple signatures with the same verifier
	for i := 0; i < 5; i++ {
		ingotID := "test-ingot-multi"
		branchHash := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
		hashCount := 3600
		timestamp := "2025-11-16T12:00:00Z"

		signatureHex, publicKeyHex, err := signer.SignPhase2Ingot(ingotID, branchHash, hashCount, timestamp)
		require.NoError(t, err, "Signing should succeed for iteration %d", i)

		err = verifier.VerifyPhase2Ingot(ingotID, branchHash, hashCount, timestamp, signatureHex, publicKeyHex)
		assert.NoError(t, err, "Verification should succeed for iteration %d", i)
	}
}

// Helper function to create a test signer
// This simulates the Refinery's FalconSigner
func createTestSigner(t *testing.T) *TestFalconSigner {
	t.Helper()

	signer, err := NewTestFalconSigner()
	require.NoError(t, err, "Failed to create test signer")

	return signer
}
