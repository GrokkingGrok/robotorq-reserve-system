// internal/refinery/hash_batch_receiver_test.go
// Tests for hash batch receiver with signature verification
//
// NOTE: These tests validate structure and logic without requiring liboqs.
// Crypto verification tests are in crypto/falcon_test.go (requires liboqs in Docker)

package refinery

import (
	"testing"
	"time"

	"b2b/refinery/internal/models"

	"github.com/stretchr/testify/assert"
)

func TestNewHashBatchReceiver(t *testing.T) {
	receiver := NewHashBatchReceiver(nil)

	assert.NotNil(t, receiver)
	assert.NotNil(t, receiver.verifier)
	assert.NotNil(t, receiver.metrics)
}

func TestReceiveHashBatch_StructureValidation(t *testing.T) {
	receiver := NewHashBatchReceiver(nil)

	tests := []struct {
		name        string
		batch       *models.HashBatchOre
		expectError bool
		errorMsg    string
	}{
		{
			name: "valid batch structure",
			batch: &models.HashBatchOre{
				ContractID:     "test-contract-001",
				DiggerID:       "test-digger-001",
				Hashes:         []string{"hash1", "hash2", "hash3"},
				HashCount:      3,
				Timestamp:      time.Now().UTC().Format(time.RFC3339),
				Signature:      "0123456789abcdef", // Invalid crypto, but structure OK
				PublicKey:      "fedcba9876543210",
				MilestoneIndex: 1,
				Joules:         100.0,
				RoboStake:      0.01,
			},
			expectError: true, // Will fail on signature verification, not structure
			errorMsg:    "signature verification failed",
		},
		{
			name: "missing contract ID",
			batch: &models.HashBatchOre{
				DiggerID:  "test-digger-001",
				Hashes:    []string{"hash1"},
				HashCount: 1,
				Timestamp: time.Now().UTC().Format(time.RFC3339),
				Signature: "sig",
				PublicKey: "pubkey",
			},
			expectError: true,
			errorMsg:    "validation failed",
		},
		{
			name: "hash count mismatch",
			batch: &models.HashBatchOre{
				ContractID: "test-contract-001",
				DiggerID:   "test-digger-001",
				Hashes:     []string{"hash1", "hash2"},
				HashCount:  5, // Wrong count
				Timestamp:  time.Now().UTC().Format(time.RFC3339),
				Signature:  "sig",
				PublicKey:  "pubkey",
			},
			expectError: true,
			errorMsg:    "validation failed",
		},
		{
			name: "empty hashes array",
			batch: &models.HashBatchOre{
				ContractID: "test-contract-001",
				DiggerID:   "test-digger-001",
				Hashes:     []string{},
				HashCount:  0,
				Timestamp:  time.Now().UTC().Format(time.RFC3339),
				Signature:  "sig",
				PublicKey:  "pubkey",
			},
			expectError: true,
			errorMsg:    "validation failed",
		},
		{
			name: "invalid timestamp format",
			batch: &models.HashBatchOre{
				ContractID: "test-contract-001",
				DiggerID:   "test-digger-001",
				Hashes:     []string{"hash1"},
				HashCount:  1,
				Timestamp:  "not-a-timestamp",
				Signature:  "sig",
				PublicKey:  "pubkey",
			},
			expectError: true,
			errorMsg:    "validation failed",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := receiver.ReceiveHashBatch(tt.batch)

			if tt.expectError {
				assert.Error(t, err)
				assert.Contains(t, err.Error(), tt.errorMsg)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

// Note: Tests for valid/invalid Falcon signatures require liboqs
// These will be run in Docker during CI
// For local testing without liboqs, the tests above verify structure validation

func TestHashBatchMetrics_Creation(t *testing.T) {
	metrics := NewHashBatchMetrics()

	assert.NotNil(t, metrics)
	assert.NotNil(t, metrics.BatchesReceivedTotal)
	assert.NotNil(t, metrics.SignaturesVerifiedTotal)
	assert.NotNil(t, metrics.SignaturesFailedTotal)
	assert.NotNil(t, metrics.HashesReceivedTotal)
	assert.NotNil(t, metrics.VerificationDuration)
}

// TODO Phase 5 Sprint 2: Add integration test with real Falcon signatures
// - Generate keypair with liboqs
// - Sign hash batch message
// - Verify signature passes
// - Verify tampered signature fails
// - Verify wrong public key fails
