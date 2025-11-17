package models

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewPhase3RoboTorqUnit_Success tests valid unit creation
func TestNewPhase3RoboTorqUnit_Success(t *testing.T) {
	merkleRoot := generateTestMerkleRoot()
	treeHeight := 10 // 1000 ingots

	unit, err := NewPhase3RoboTorqUnit(merkleRoot, treeHeight)

	require.NoError(t, err)
	assert.NotNil(t, unit)
	assert.NotEmpty(t, unit.UnitID)
	assert.Equal(t, merkleRoot, unit.MerkleRoot)
	assert.Equal(t, treeHeight, unit.TreeHeight)
	assert.NotEmpty(t, unit.MerkleProofAPI)
	assert.False(t, unit.MintedAt.IsZero())

	t.Logf("Created Phase3RoboTorqUnit: %s", unit.UnitID)
	t.Logf("Size: %d bytes", unit.SizeBytes())
}

// TestNewPhase3RoboTorqUnit_InvalidMerkleRoot tests invalid merkle root rejection
func TestNewPhase3RoboTorqUnit_InvalidMerkleRoot(t *testing.T) {
	tests := []struct {
		name       string
		merkleRoot string
		errorMsg   string
	}{
		{"too short", "short", "invalid merkle_root length"},
		{"too long", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "invalid merkle_root length"},
		{"empty", "", "invalid merkle_root length"},
		{"invalid hex", "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz", "invalid merkle_root"},
		{"non-hex chars", "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg", "invalid merkle_root"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			unit, err := NewPhase3RoboTorqUnit(tt.merkleRoot, 10)

			assert.Error(t, err)
			assert.Nil(t, unit)
			assert.Contains(t, err.Error(), tt.errorMsg)
		})
	}
}

// TestNewPhase3RoboTorqUnit_HexValidation tests hex character validation
func TestNewPhase3RoboTorqUnit_HexValidation(t *testing.T) {
	tests := []struct {
		name       string
		merkleRoot string
		shouldPass bool
	}{
		{"lowercase hex", "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890", true},
		{"uppercase hex", "ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890", true},
		{"mixed case hex", "AbCdEf1234567890aBcDeF1234567890AbCdEf1234567890aBcDeF1234567890", true},
		{"all zeros", "0000000000000000000000000000000000000000000000000000000000000000", true},
		{"all f's", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", true},
		{"with space", "abcdef1234567890 bcdef1234567890abcdef1234567890abcdef1234567890", false},
		{"with dash", "abcdef1234567890-bcdef1234567890abcdef1234567890abcdef1234567890", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			unit, err := NewPhase3RoboTorqUnit(tt.merkleRoot, 10)

			if tt.shouldPass {
				assert.NoError(t, err)
				assert.NotNil(t, unit)
			} else {
				assert.Error(t, err)
				assert.Nil(t, unit)
			}
		})
	}
}

// TestPhase3RoboTorqUnit_Validate tests validation method
func TestPhase3RoboTorqUnit_Validate(t *testing.T) {
	// Valid unit
	validUnit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	err = validUnit.Validate()
	assert.NoError(t, err, "valid unit should pass validation")

	// Test invalid cases
	tests := []struct {
		name     string
		modify   func(*Phase3RoboTorqUnit)
		errorMsg string
	}{
		{
			"empty unit_id",
			func(u *Phase3RoboTorqUnit) { u.UnitID = "" },
			"unit_id is required",
		},
		{
			"invalid merkle_root length",
			func(u *Phase3RoboTorqUnit) { u.MerkleRoot = "short" },
			"merkle_root must be 64-char hex",
		},
		{
			"invalid merkle_root hex",
			func(u *Phase3RoboTorqUnit) {
				u.MerkleRoot = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
			},
			"merkle_root must be hex string",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Create fresh valid unit
			unit, _ := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)

			// Apply modification
			tt.modify(unit)

			// Validate should fail
			err := unit.Validate()
			assert.Error(t, err)
			assert.Contains(t, err.Error(), tt.errorMsg)
		})
	}
}

// TestPhase3RoboTorqUnit_ToJSON tests JSON serialization
func TestPhase3RoboTorqUnit_ToJSON(t *testing.T) {
	unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	data, err := unit.ToJSON()

	require.NoError(t, err)
	assert.NotEmpty(t, data)
	assert.Contains(t, string(data), "unit_id")
	assert.Contains(t, string(data), "merkle_root")
	assert.Contains(t, string(data), "tree_height")      // NEW field for proof verification
	assert.Contains(t, string(data), "merkle_proof_api") // NEW field for proof retrieval
	assert.Contains(t, string(data), "minted_at")

	// Should NOT contain old metadata fields (contracts, diggers, etc.)
	assert.NotContains(t, string(data), "total_joules")
	assert.NotContains(t, string(data), "contract_ids")
	assert.NotContains(t, string(data), "digger_ids")
	assert.NotContains(t, string(data), "refinery_ids")

	t.Logf("JSON size: %d bytes", len(data))
}

// TestPhase3RoboTorqUnit_SizeBytes tests size estimation
func TestPhase3RoboTorqUnit_SizeBytes(t *testing.T) {
	unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	size := unit.SizeBytes()

	assert.Greater(t, size, 0, "size should be positive")
	assert.Less(t, size, 300, "size should be <300 bytes (with tree_height + merkle_proof_api)")

	t.Logf("Phase3RoboTorqUnit size: %d bytes", size)
}

// TestPhase3RoboTorqUnit_NFCCompatibility tests NFC tag size requirements
func TestPhase3RoboTorqUnit_NFCCompatibility(t *testing.T) {
	unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	// NTAG215 capacity: 540 bytes usable
	// Required for redemption:
	//   - serial_number: 25 bytes
	//   - denomination: 8 bytes
	//   - merkle_root: 64 bytes (from unit)
	//   - signature: 64 bytes (Ed25519)
	//   - public_key: 64 bytes
	//   Total: ~257 bytes minimum

	unitSize := unit.SizeBytes()
	nfcCapacity := 540
	minRequiredData := 257 // serial + denomination + signature + pubkey

	totalNFCUsage := minRequiredData + (unitSize - 64) // Subtract merkle_root (already counted)

	t.Logf("Unit size: %d bytes", unitSize)
	t.Logf("NFC capacity: %d bytes", nfcCapacity)
	t.Logf("Min required data: %d bytes", minRequiredData)
	t.Logf("Total NFC usage: %d bytes", totalNFCUsage)
	t.Logf("Remaining: %d bytes", nfcCapacity-totalNFCUsage)

	assert.Less(t, totalNFCUsage, nfcCapacity, "Unit must fit in NFC tag with signature")
	assert.Greater(t, nfcCapacity-totalNFCUsage, 100, "Should have >100 bytes spare")
}

// TestPhase3RoboTorqUnit_UniqueIDs tests ID generation uniqueness
func TestPhase3RoboTorqUnit_UniqueIDs(t *testing.T) {
	unit1, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	// Sleep briefly to ensure different timestamps
	time.Sleep(1 * time.Millisecond)

	unit2, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	assert.NotEqual(t, unit1.UnitID, unit2.UnitID, "unit IDs should be unique")
}

// TestPhase3RoboTorqUnit_ComparisonWithOldFormat tests size comparison
func TestPhase3RoboTorqUnit_ComparisonWithOldFormat(t *testing.T) {
	// Phase 3 unit (pure hash-only)
	phase3Unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10)
	require.NoError(t, err)

	phase3Size := phase3Unit.SizeBytes()

	// Old format estimated size (with metadata arrays):
	//   - UnitID: 36 bytes (UUID)
	//   - MerkleRoot: 64 bytes
	//   - TreeHeight: 8 bytes
	//   - TotalJoules: 8 bytes
	//   - TotalRoboStake: 8 bytes
	//   - ContractIDs: ~100 bytes (5 contracts × 20 bytes)
	//   - DiggerIDs: ~60 bytes (3 diggers × 20 bytes)
	//   - RefineryIDs: ~20 bytes (1 refinery × 20 bytes)
	//   - IngotCount: 8 bytes
	//   - MintedAt: 8 bytes
	//   Total: ~350-500 bytes
	estimatedOldSize := 450

	sizeReduction := float64(estimatedOldSize-phase3Size) / float64(estimatedOldSize) * 100
	sizeMultiplier := float64(estimatedOldSize) / float64(phase3Size)

	t.Logf("Phase 3 size (pure hash-only): %d bytes", phase3Size)
	t.Logf("Estimated old format size (with metadata): %d bytes", estimatedOldSize)
	t.Logf("Size reduction: %.1f%% (%.1fx smaller)", sizeReduction, sizeMultiplier)

	assert.Less(t, phase3Size, 300, "Phase 3 should be <300 bytes (with proof metadata)")
	assert.Greater(t, estimatedOldSize, 300, "Old format should be >300 bytes")
	assert.Greater(t, sizeMultiplier, 1.5, "Should be at least 1.5x smaller")
}

// generateTestMerkleRoot creates a 64-char hex test merkle root
func generateTestMerkleRoot() string {
	return "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
