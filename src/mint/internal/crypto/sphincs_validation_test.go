package crypto

import (
	"log/slog"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// getTestLogger creates a logger for tests
func getTestLogger() *slog.Logger {
	return slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
}

// TestSPHINCSPlusSigner_SignAndVerify_RoundTrip tests full signature cycle
func TestSPHINCSPlusSigner_SignAndVerify_RoundTrip(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err, "Failed to create signer")

	unitID := "RT-20251117-test-001"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	// Sign
	signature, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err, "Signing failed")
	require.NotEmpty(t, signature, "Signature should not be empty")
	require.NotEmpty(t, publicKey, "Public key should not be empty")

	t.Logf("Signature length: %d hex chars (~%d bytes)", len(signature), len(signature)/2)
	t.Logf("Public key length: %d hex chars (%d bytes)", len(publicKey), len(publicKey)/2)

	// Verify with correct data
	err = signer.VerifyPhase3Unit(unitID, merkleRoot, mintedAt, signature, publicKey)
	assert.NoError(t, err, "Verification should succeed with correct data")
}

// TestSPHINCSPlusSigner_VerifyFails_TamperedUnitID tests tampered unit ID detection
func TestSPHINCSPlusSigner_VerifyFails_TamperedUnitID(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-20251117-test-002"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	signature, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err)

	// Tamper with unit ID
	tamperedUnitID := "RT-20251117-HACKED-002"

	err = signer.VerifyPhase3Unit(tamperedUnitID, merkleRoot, mintedAt, signature, publicKey)
	assert.Error(t, err, "Verification should fail with tampered unit ID")
	assert.Contains(t, err.Error(), "invalid SPHINCS+ signature", "Error should mention invalid signature")
}

// TestSPHINCSPlusSigner_VerifyFails_TamperedMerkleRoot tests tampered merkle root detection
func TestSPHINCSPlusSigner_VerifyFails_TamperedMerkleRoot(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-20251117-test-003"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	signature, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err)

	// Tamper with merkle root (change one character)
	tamperedMerkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccde"

	err = signer.VerifyPhase3Unit(unitID, tamperedMerkleRoot, mintedAt, signature, publicKey)
	assert.Error(t, err, "Verification should fail with tampered merkle root")
}

// TestSPHINCSPlusSigner_VerifyFails_TamperedTimestamp tests tampered timestamp detection
func TestSPHINCSPlusSigner_VerifyFails_TamperedTimestamp(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-20251117-test-004"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	signature, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err)

	// Tamper with timestamp
	tamperedMintedAt := "2025-11-17T13:00:00Z" // Changed hour

	err = signer.VerifyPhase3Unit(unitID, merkleRoot, tamperedMintedAt, signature, publicKey)
	assert.Error(t, err, "Verification should fail with tampered timestamp")
}

// TestSPHINCSPlusSigner_VerifyFails_InvalidSignature tests invalid signature detection
func TestSPHINCSPlusSigner_VerifyFails_InvalidSignature(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-20251117-test-005"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	_, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err)

	// Use completely different signature
	invalidSignature := "0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff"

	err = signer.VerifyPhase3Unit(unitID, merkleRoot, mintedAt, invalidSignature, publicKey)
	assert.Error(t, err, "Verification should fail with invalid signature")
}

// TestSPHINCSPlusSigner_VerifyFails_WrongPublicKey tests wrong public key detection
func TestSPHINCSPlusSigner_VerifyFails_WrongPublicKey(t *testing.T) {
	signer1, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	signer2, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-20251117-test-006"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	// Sign with signer1
	signature, _, err := signer1.SignPhase3Unit(unitID, merkleRoot, mintedAt)
	require.NoError(t, err)

	// Try to verify with signer2's public key
	wrongPublicKey := signer2.GetPublicKey()

	err = signer1.VerifyPhase3Unit(unitID, merkleRoot, mintedAt, signature, wrongPublicKey)
	assert.Error(t, err, "Verification should fail with wrong public key")
}

// TestSPHINCSPlusSigner_MultipleSignatures tests signing multiple messages
func TestSPHINCSPlusSigner_MultipleSignatures(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	testCases := []struct {
		unitID     string
		merkleRoot string
		mintedAt   string
	}{
		{"RT-001", "aaaa", "2025-11-17T10:00:00Z"},
		{"RT-002", "bbbb", "2025-11-17T11:00:00Z"},
		{"RT-003", "cccc", "2025-11-17T12:00:00Z"},
	}

	publicKey := signer.GetPublicKey()

	for _, tc := range testCases {
		t.Run(tc.unitID, func(t *testing.T) {
			signature, returnedPubKey, err := signer.SignPhase3Unit(tc.unitID, tc.merkleRoot, tc.mintedAt)
			require.NoError(t, err)
			assert.Equal(t, publicKey, returnedPubKey, "Public key should be consistent")

			err = signer.VerifyPhase3Unit(tc.unitID, tc.merkleRoot, tc.mintedAt, signature, publicKey)
			assert.NoError(t, err, "Each signature should verify independently")
		})
	}
}

// TestSPHINCSPlusSigner_SignatureUniqueness tests that different messages produce different signatures
func TestSPHINCSPlusSigner_SignatureUniqueness(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	sig1, _, err := signer.SignPhase3Unit("RT-001", merkleRoot, mintedAt)
	require.NoError(t, err)

	sig2, _, err := signer.SignPhase3Unit("RT-002", merkleRoot, mintedAt)
	require.NoError(t, err)

	assert.NotEqual(t, sig1, sig2, "Different unit IDs should produce different signatures")
}

// TestSPHINCSPlusSigner_PublicKeyConsistency tests public key remains the same
func TestSPHINCSPlusSigner_PublicKeyConsistency(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	pubKey1 := signer.GetPublicKey()
	pubKey2 := signer.GetPublicKey()

	assert.Equal(t, pubKey1, pubKey2, "GetPublicKey should return consistent value")
	assert.Len(t, pubKey1, 64, "SPHINCS+-SHA2-128f-simple public key should be 64 hex chars (32 bytes)")
}

// TestSPHINCSPlusSigner_EmptyInputs tests error handling for empty inputs
func TestSPHINCSPlusSigner_EmptyInputs(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	testCases := []struct {
		name       string
		unitID     string
		merkleRoot string
		mintedAt   string
	}{
		{"empty unit ID", "", "aabbccdd", "2025-11-17T12:00:00Z"},
		{"empty merkle root", "RT-001", "", "2025-11-17T12:00:00Z"},
		{"empty timestamp", "RT-001", "aabbccdd", ""},
		{"all empty", "", "", ""},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			// Should still sign (no validation at crypto layer)
			signature, publicKey, err := signer.SignPhase3Unit(tc.unitID, tc.merkleRoot, tc.mintedAt)

			// But signature should be generated
			assert.NoError(t, err, "Signing should succeed even with empty inputs")
			assert.NotEmpty(t, signature, "Should generate signature")

			// And verification should work with same empty inputs
			err = signer.VerifyPhase3Unit(tc.unitID, tc.merkleRoot, tc.mintedAt, signature, publicKey)
			assert.NoError(t, err, "Verification should succeed with matching empty inputs")
		})
	}
}

// TestSPHINCSPlusSigner_SignatureFormat tests signature format
func TestSPHINCSPlusSigner_SignatureFormat(t *testing.T) {
	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	signature, _, err := signer.SignPhase3Unit("RT-001", "aaaa", "2025-11-17T12:00:00Z")
	require.NoError(t, err)

	// SPHINCS+-SHA2-128f-simple produces ~17KB signatures
	// Hex encoded = 2 chars per byte = ~34KB hex chars
	assert.Greater(t, len(signature), 30000, "Signature should be large (~34K hex chars)")
	assert.Less(t, len(signature), 40000, "Signature should not exceed expected size")

	// Should be valid hex
	for _, char := range signature {
		assert.True(t, (char >= '0' && char <= '9') || (char >= 'a' && char <= 'f'),
			"Signature should only contain hex characters")
	}
}

// TestSPHINCSPlusSigner_Performance benchmarks signing and verification
func TestSPHINCSPlusSigner_Performance(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	signer, err := NewSPHINCSPlusSigner(getTestLogger())
	require.NoError(t, err)

	unitID := "RT-perf-test"
	merkleRoot := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	mintedAt := "2025-11-17T12:00:00Z"

	// Test signing performance
	iterations := 10

	for i := 0; i < iterations; i++ {
		signature, publicKey, err := signer.SignPhase3Unit(unitID, merkleRoot, mintedAt)
		require.NoError(t, err)

		err = signer.VerifyPhase3Unit(unitID, merkleRoot, mintedAt, signature, publicKey)
		require.NoError(t, err)
	}

	t.Logf("Completed %d sign+verify cycles", iterations)
}
