//go:build cgo
// +build cgo

package crypto

import (
	"encoding/hex"
	"fmt"

	"github.com/open-quantum-safe/liboqs-go/oqs"
)

// TestFalconSigner is a test helper that mimics Refinery's FalconSigner
// Used in unit tests to generate real Falcon-1024 signatures
type TestFalconSigner struct {
	secretKey []byte
	publicKey []byte
}

// NewTestFalconSigner creates a new test signer with a fresh keypair
func NewTestFalconSigner() (*TestFalconSigner, error) {
	sig := oqs.Signature{}
	defer sig.Clean()

	if err := sig.Init("Falcon-1024", nil); err != nil {
		return nil, fmt.Errorf("failed to initialize Falcon-1024: %w", err)
	}

	publicKey, err := sig.GenerateKeyPair()
	if err != nil {
		return nil, fmt.Errorf("failed to generate keypair: %w", err)
	}

	// Export and COPY secret key (critical for multiple signatures)
	exportedKey := sig.ExportSecretKey()
	secretKey := make([]byte, len(exportedKey))
	copy(secretKey, exportedKey)

	return &TestFalconSigner{
		secretKey: secretKey,
		publicKey: publicKey,
	}, nil
}

// SignPhase2Ingot signs a Phase2Ingot using Falcon-1024
// This mimics the Refinery's signing logic
func (ts *TestFalconSigner) SignPhase2Ingot(
	ingotID string,
	branchHash string,
	hashCount int,
	timestamp string,
) (signatureHex string, publicKeyHex string, err error) {
	// Construct message (must match Mint verification format)
	message := fmt.Sprintf("%s|%s|%d|%s",
		ingotID,
		branchHash,
		hashCount,
		timestamp)

	// Create fresh signature object
	sig := oqs.Signature{}
	defer sig.Clean()

	// CRITICAL: Copy secret key before passing to Init()
	// Init() stores a reference, Clean() zeroes it
	secretKeyCopy := make([]byte, len(ts.secretKey))
	copy(secretKeyCopy, ts.secretKey)

	if err := sig.Init("Falcon-1024", secretKeyCopy); err != nil {
		return "", "", fmt.Errorf("failed to init: %w", err)
	}

	signature, err := sig.Sign([]byte(message))
	if err != nil {
		return "", "", fmt.Errorf("failed to sign: %w", err)
	}

	return hex.EncodeToString(signature), hex.EncodeToString(ts.publicKey), nil
}

// Clean securely erases the secret key
func (ts *TestFalconSigner) Clean() {
	if ts.secretKey != nil {
		oqs.MemCleanse(ts.secretKey)
		ts.secretKey = nil
	}
}
