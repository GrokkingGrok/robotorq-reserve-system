//go:build cgo
// +build cgo

package crypto

import (
	"encoding/hex"
	"fmt"
	"testing"

	"github.com/open-quantum-safe/liboqs-go/oqs"
)

// TestSPHINCS_MinimalRoundTrip tests basic liboqs SPHINCS+ functionality
func TestSPHINCS_MinimalRoundTrip(t *testing.T) {
	// Generate keypair
	sigGen := oqs.Signature{}
	defer sigGen.Clean()

	if err := sigGen.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Init for keygen failed: %v", err)
	}

	publicKey, err := sigGen.GenerateKeyPair()
	if err != nil {
		t.Fatalf("GenerateKeyPair failed: %v", err)
	}

	privateKey := sigGen.ExportSecretKey()
	t.Logf("Generated public key: %d bytes", len(publicKey))
	t.Logf("Generated private key: %d bytes", len(privateKey))

	// Sign a message
	message := []byte("test message")

	// CRITICAL: Copy private key before using it
	privateKeyCopy := make([]byte, len(privateKey))
	copy(privateKeyCopy, privateKey)

	sigSign := oqs.Signature{}
	defer sigSign.Clean()

	if err := sigSign.Init("SPHINCS+-SHA2-128f-simple", privateKeyCopy); err != nil {
		t.Fatalf("Init for signing failed: %v", err)
	}

	signature, err := sigSign.Sign(message)
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	t.Logf("Generated signature: %d bytes", len(signature))
	t.Logf("Signature hex (first 64 chars): %s", hex.EncodeToString(signature)[:64])

	// Verify the signature
	sigVerify := oqs.Signature{}
	defer sigVerify.Clean()

	if err := sigVerify.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Init for verification failed: %v", err)
	}

	isValid, err := sigVerify.Verify(message, signature, publicKey)
	if err != nil {
		t.Fatalf("Verify returned error: %v", err)
	}

	if !isValid {
		t.Fatalf("Verify returned false - signature is INVALID")
	}

	t.Log("SUCCESS: Signature verified!")
}

// TestSPHINCS_WithFormatting tests with hex encoding like real code
func TestSPHINCS_WithFormatting(t *testing.T) {
	// Generate keypair
	sigGen := oqs.Signature{}
	defer sigGen.Clean()

	if err := sigGen.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Init failed: %v", err)
	}

	publicKey, err := sigGen.GenerateKeyPair()
	if err != nil {
		t.Fatalf("GenerateKeyPair failed: %v", err)
	}

	privateKey := sigGen.ExportSecretKey()

	// Sign - WITH HEX ENCODING
	message := "unit-001|merkle-root|2025-11-17T12:00:00Z"

	privateKeyCopy := make([]byte, len(privateKey))
	copy(privateKeyCopy, privateKey)

	sigSign := oqs.Signature{}
	defer sigSign.Clean()

	if err := sigSign.Init("SPHINCS+-SHA2-128f-simple", privateKeyCopy); err != nil {
		t.Fatalf("Init for signing failed: %v", err)
	}

	signatureBytes, err := sigSign.Sign([]byte(message))
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	// Convert to hex (like real code)
	signatureHex := hex.EncodeToString(signatureBytes)
	publicKeyHex := hex.EncodeToString(publicKey)

	t.Logf("Signature hex length: %d", len(signatureHex))
	t.Logf("Public key hex length: %d", len(publicKeyHex))

	// Verify - DECODE FROM HEX
	sigBytes, err := hex.DecodeString(signatureHex)
	if err != nil {
		t.Fatalf("Failed to decode signature hex: %v", err)
	}

	pubKeyBytes, err := hex.DecodeString(publicKeyHex)
	if err != nil {
		t.Fatalf("Failed to decode public key hex: %v", err)
	}

	sigVerify := oqs.Signature{}
	defer sigVerify.Clean()

	if err := sigVerify.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Init for verification failed: %v", err)
	}

	isValid, err := sigVerify.Verify([]byte(message), sigBytes, pubKeyBytes)
	if err != nil {
		t.Fatalf("Verify returned error: %v", err)
	}

	if !isValid {
		t.Fatalf("Verify returned false")
	}

	t.Log("SUCCESS: Hex-encoded signature verified!")
}

// TestSPHINCS_ExactlyLikeRealCode mimics our actual implementation
func TestSPHINCS_ExactlyLikeRealCode(t *testing.T) {
	// This is EXACTLY how NewSPHINCSPlusSigner works
	sigGen := oqs.Signature{}
	defer sigGen.Clean()

	if err := sigGen.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Init failed: %v", err)
	}

	publicKey, err := sigGen.GenerateKeyPair()
	if err != nil {
		t.Fatalf("GenerateKeyPair failed: %v", err)
	}

	privateKey := sigGen.ExportSecretKey()

	t.Logf("Keypair generated")

	// This is EXACTLY how SignPhase3Unit works
	message := fmt.Sprintf("%s|%s|%s", "unit-001", "merkle-root-hash", "2025-11-17T12:00:00Z")

	// Sign with private key copy (our fix)
	privateKeyCopy := make([]byte, len(privateKey))
	copy(privateKeyCopy, privateKey)

	sigSign := oqs.Signature{}
	defer sigSign.Clean()

	if err := sigSign.Init("SPHINCS+-SHA2-128f-simple", privateKeyCopy); err != nil {
		t.Fatalf("Sign init failed: %v", err)
	}

	signature, err := sigSign.Sign([]byte(message))
	if err != nil {
		t.Fatalf("Sign failed: %v", err)
	}

	signatureHex := hex.EncodeToString(signature)
	publicKeyHex := hex.EncodeToString(publicKey)

	t.Logf("Signed message")

	// This is EXACTLY how VerifyPhase3Unit works
	publicKeyDecoded, err := hex.DecodeString(publicKeyHex)
	if err != nil {
		t.Fatalf("Decode public key failed: %v", err)
	}

	signatureDecoded, err := hex.DecodeString(signatureHex)
	if err != nil {
		t.Fatalf("Decode signature failed: %v", err)
	}

	sigVerify := oqs.Signature{}
	defer sigVerify.Clean()

	if err := sigVerify.Init("SPHINCS+-SHA2-128f-simple", nil); err != nil {
		t.Fatalf("Verify init failed: %v", err)
	}

	isValid, err := sigVerify.Verify([]byte(message), signatureDecoded, publicKeyDecoded)
	if err != nil {
		t.Fatalf("Verify error: %v", err)
	}

	if !isValid {
		t.Fatalf("Verification FAILED")
	}

	t.Log("SUCCESS: Exact real-code flow verified!")
}
