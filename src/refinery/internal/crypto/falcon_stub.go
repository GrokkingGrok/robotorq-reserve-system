//go:build !cgo
// +build !cgo

package crypto

import (
	"fmt"
)

// FalconSigner handles Falcon-1024 signature generation for Phase2Ingots
//
// STUB IMPLEMENTATION - Used when liboqs-go is not available (local dev)
// =======================================================================
// This stub always returns errors. Real crypto requires CGO and liboqs.
// Run tests in Docker or with CGO_ENABLED=1 and liboqs installed.
type FalconSigner struct {
	secretKey []byte
	publicKey []byte
}

// NewFalconSigner creates a stub signer that returns errors
func NewFalconSigner() (*FalconSigner, error) {
	return nil, fmt.Errorf("FalconSigner requires CGO and liboqs-go (run in Docker or install liboqs)")
}

// Clean is a no-op in stub
func (fs *FalconSigner) Clean() {
	// No-op
}

// SignPhase2Ingot returns an error in stub implementation
func (fs *FalconSigner) SignPhase2Ingot(
	ingotID string,
	branchHash string,
	hashCount int,
	timestamp string,
) (signatureHex string, publicKeyHex string, error error) {
	return "", "", fmt.Errorf("SignPhase2Ingot requires CGO and liboqs-go (run in Docker)")
}

// FalconVerifier handles Falcon-1024 signature verification
//
// STUB IMPLEMENTATION - Used when liboqs-go is not available (local dev)
type FalconVerifier struct {
}

// NewFalconVerifier creates a stub verifier
func NewFalconVerifier() *FalconVerifier {
	return &FalconVerifier{}
}

// VerifyHashBatch returns an error in stub implementation
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
	return fmt.Errorf("VerifyHashBatch requires CGO and liboqs-go (run in Docker)")
}
