//go:build cgo
// +build cgo

package crypto

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

// NOTE: These tests require liboqs C library (pkg-config)
// They will be skipped locally but run in Docker CI
// See CRYPTO_SETUP.md for installation instructions

func TestVerifyHashBatch_ValidSignature(t *testing.T) {
	t.Skip("TODO: Requires liboqs C library installation - see CRYPTO_SETUP.md")

	verifier := NewFalconVerifier()

	// Test data from Digger integration test
	contractID := "test-contract-001"
	diggerID := "test-digger-001"
	milestoneIndex := uint32(0)
	joules := 0.0
	roboStake := 0.0
	unitHashes := []string{
		"hash1",
		"hash2",
		"hash3",
	}
	timestamp := "2025-11-16T12:00:00Z"

	// TODO: Get real signature + public key from Digger test
	signatureHex := "PLACEHOLDER"
	publicKeyHex := "PLACEHOLDER"

	err := verifier.VerifyHashBatch(
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

	assert.NoError(t, err)
}

func TestVerifyHashBatch_InvalidSignature(t *testing.T) {
	t.Skip("TODO: Requires liboqs C library installation - see CRYPTO_SETUP.md")

	verifier := NewFalconVerifier()

	contractID := "test-contract-001"
	diggerID := "test-digger-001"
	milestoneIndex := uint32(0)
	joules := 0.0
	roboStake := 0.0
	unitHashes := []string{"hash1", "hash2"}
	timestamp := "2025-11-16T12:00:00Z"

	// Invalid signature (wrong bytes)
	signatureHex := "0000000000"
	publicKeyHex := "0000000000"

	err := verifier.VerifyHashBatch(
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

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid")
}

func TestVerifyHashBatch_InvalidHex(t *testing.T) {
	verifier := NewFalconVerifier()

	// This test doesn't need liboqs, just tests hex validation
	err := verifier.VerifyHashBatch(
		"contract",
		"digger",
		0,
		0.0,
		0.0,
		[]string{"hash1"},
		"2025-11-16T12:00:00Z",
		"NOT_HEX", // Invalid hex
		"",
	)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid signature hex")
}
