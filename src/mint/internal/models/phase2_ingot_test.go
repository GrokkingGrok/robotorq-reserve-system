package models

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

// TestPhase2Ingot_Validate_Success tests valid Phase2Ingot
func TestPhase2Ingot_Validate_Success(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "test-ingot-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.0,
		Timestamp:      time.Now(),
		Signature:      "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		PublicKey:      "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
	}

	err := ingot.Validate()
	assert.NoError(t, err, "valid ingot should pass validation")
}

// TestPhase2Ingot_Validate_InvalidBranchHashLength tests branch hash length validation
func TestPhase2Ingot_Validate_InvalidBranchHashLength(t *testing.T) {
	tests := []struct {
		name       string
		branchHash string
		errorMsg   string
	}{
		{"too short", "aaaa", "invalid branch_hash length"},
		{"too long", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "invalid branch_hash length"},
		{"empty", "", "invalid branch_hash length"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ingot := createValidPhase2Ingot()
			ingot.BranchHash = tt.branchHash

			err := ingot.Validate()
			assert.Error(t, err)
			assert.Contains(t, err.Error(), tt.errorMsg)
		})
	}
}

// TestPhase2Ingot_Validate_InvalidBranchHashHex tests branch hash hex validation
func TestPhase2Ingot_Validate_InvalidBranchHashHex(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.BranchHash = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not valid hex")
}

// TestPhase2Ingot_Validate_InvalidHashCount tests hash count validation
func TestPhase2Ingot_Validate_InvalidHashCount(t *testing.T) {
	tests := []struct {
		name      string
		hashCount int
	}{
		{"zero", 0},
		{"too small", 100},
		{"too large", 5000},
		{"negative", -1},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ingot := createValidPhase2Ingot()
			ingot.HashCount = tt.hashCount

			err := ingot.Validate()
			assert.Error(t, err)
			assert.Contains(t, err.Error(), "invalid hash_count")
		})
	}
}

// TestPhase2Ingot_Validate_EmptyContractIDs tests contract IDs validation
func TestPhase2Ingot_Validate_EmptyContractIDs(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.ContractIDs = []string{}

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "contract_ids cannot be empty")
}

// TestPhase2Ingot_Validate_EmptyDiggerIDs tests digger IDs validation
func TestPhase2Ingot_Validate_EmptyDiggerIDs(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.DiggerIDs = []string{}

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "digger_ids cannot be empty")
}

// TestPhase2Ingot_Validate_EmptyID tests ID validation
func TestPhase2Ingot_Validate_EmptyID(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.ID = ""

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "id cannot be empty")
}

// TestPhase2Ingot_Validate_ZeroTimestamp tests zero timestamp validation
func TestPhase2Ingot_Validate_ZeroTimestamp(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Timestamp = time.Time{}

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "timestamp cannot be zero")
}

// TestPhase2Ingot_Validate_FutureTimestamp tests future timestamp validation
func TestPhase2Ingot_Validate_FutureTimestamp(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Timestamp = time.Now().Add(2 * time.Hour)

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "timestamp is too far in future")
}

// TestPhase2Ingot_Validate_EmptySignature tests signature validation
func TestPhase2Ingot_Validate_EmptySignature(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Signature = ""

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "signature cannot be empty")
}

// TestPhase2Ingot_Validate_InvalidSignatureHex tests signature hex validation
func TestPhase2Ingot_Validate_InvalidSignatureHex(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Signature = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "signature is not valid hex")
}

// TestPhase2Ingot_Validate_EmptyPublicKey tests public key validation
func TestPhase2Ingot_Validate_EmptyPublicKey(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.PublicKey = ""

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "public_key cannot be empty")
}

// TestPhase2Ingot_Validate_InvalidPublicKeyHex tests public key hex validation
func TestPhase2Ingot_Validate_InvalidPublicKeyHex(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.PublicKey = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"

	err := ingot.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "public_key is not valid hex")
}

// TestPhase2Ingot_Validate_MultipleContracts tests multiple contracts
func TestPhase2Ingot_Validate_MultipleContracts(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.ContractIDs = []string{"contract-1", "contract-2", "contract-3"}

	err := ingot.Validate()
	assert.NoError(t, err)
}

// TestPhase2Ingot_Validate_MultipleDiggers tests multiple diggers
func TestPhase2Ingot_Validate_MultipleDiggers(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.DiggerIDs = []string{"digger-1", "digger-2", "digger-3"}

	err := ingot.Validate()
	assert.NoError(t, err)
}

// TestPhase2Ingot_Validate_RecentTimestamp tests recent timestamp is valid
func TestPhase2Ingot_Validate_RecentTimestamp(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Timestamp = time.Now().Add(-5 * time.Second)

	err := ingot.Validate()
	assert.NoError(t, err)
}

// TestPhase2Ingot_Validate_OldTimestamp tests old timestamp is valid
func TestPhase2Ingot_Validate_OldTimestamp(t *testing.T) {
	ingot := createValidPhase2Ingot()
	ingot.Timestamp = time.Now().Add(-24 * time.Hour)

	err := ingot.Validate()
	assert.NoError(t, err)
}

// TestPhase2Ingot_Validate_ValidBranchHashFormats tests various valid branch hash formats
func TestPhase2Ingot_Validate_ValidBranchHashFormats(t *testing.T) {
	tests := []struct {
		name       string
		branchHash string
	}{
		{"lowercase hex", "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"},
		{"uppercase hex", "ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890"},
		{"mixed case", "AbCdEf1234567890aBcDeF1234567890AbCdEf1234567890aBcDeF1234567890"},
		{"all zeros", "0000000000000000000000000000000000000000000000000000000000000000"},
		{"all f's", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ingot := createValidPhase2Ingot()
			ingot.BranchHash = tt.branchHash

			err := ingot.Validate()
			assert.NoError(t, err)
		})
	}
}

// TestPhase2Ingot_Validate_EdgeCaseTimestamp tests timestamp exactly at 1 hour boundary
func TestPhase2Ingot_Validate_EdgeCaseTimestamp(t *testing.T) {
	ingot := createValidPhase2Ingot()
	// Just under 1 hour should pass
	ingot.Timestamp = time.Now().Add(59 * time.Minute)

	err := ingot.Validate()
	assert.NoError(t, err)
}

// ============================================================================
// Helper Functions
// ============================================================================

// createValidPhase2Ingot creates a valid Phase2Ingot for testing
func createValidPhase2Ingot() *Phase2Ingot {
	return &Phase2Ingot{
		ID:             "test-ingot-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.0,
		Timestamp:      time.Now(),
		Signature:      "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		PublicKey:      "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
	}
}

// createPhase2IngotWithSignature creates Phase2Ingot with specific signature
func createPhase2IngotWithSignature(sig string) *Phase2Ingot {
	ingot := createValidPhase2Ingot()
	ingot.Signature = sig
	return ingot
}
