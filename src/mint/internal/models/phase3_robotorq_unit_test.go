package models

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// generateTestMerkleRoot returns a fixed valid 64-char hex root for tests.
func generateTestMerkleRoot() string {
	return "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}

// TestNewPhase3RoboTorqUnit_Success ensures constructor populates required fields.
func TestNewPhase3RoboTorqUnit_Success(t *testing.T) {
	root := generateTestMerkleRoot()
	unit, err := NewPhase3RoboTorqUnit(root, 10, 5.0, []string{"contract-1", "contract-2"})
	require.NoError(t, err)
	assert.NotEmpty(t, unit.UnitID)
	assert.Equal(t, root, unit.MerkleRoot)
	assert.Equal(t, 10, unit.TreeHeight)
	assert.InDelta(t, 5.0, unit.RoboStakeTotal, 0.0000001)
	assert.Equal(t, []string{"contract-1", "contract-2"}, unit.ContractIDs)
	assert.Contains(t, unit.MerkleProofAPI, unit.UnitID)
	assert.False(t, unit.MintedAt.IsZero())
}

// TestNewPhase3RoboTorqUnit_InvalidMerkleRoot covers bad length and non-hex chars.
func TestNewPhase3RoboTorqUnit_InvalidMerkleRoot(t *testing.T) {
	cases := []string{
		"",      // empty
		"short", // too short
		"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // too long
		"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",         // invalid chars
	}
	for _, c := range cases {
		_, err := NewPhase3RoboTorqUnit(c, 10, 1.0, []string{"c"})
		assert.Error(t, err, c)
	}
}

// TestPhase3RoboTorqUnit_ValidateFailures mutates a valid unit to provoke errors.
func TestPhase3RoboTorqUnit_ValidateFailures(t *testing.T) {
	unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10, 1.0, []string{"c"})
	require.NoError(t, err)

	// Empty unit_id
	originalID := unit.UnitID
	unit.UnitID = ""
	assert.Error(t, unit.Validate())
	unit.UnitID = originalID

	// Bad merkle_root length
	originalRoot := unit.MerkleRoot
	unit.MerkleRoot = "abc"
	assert.Error(t, unit.Validate())
	unit.MerkleRoot = originalRoot

	// Bad tree_height
	unit.TreeHeight = 999
	assert.Error(t, unit.Validate())
	unit.TreeHeight = 10

	// Empty merkle_proof_api
	unit.MerkleProofAPI = ""
	assert.Error(t, unit.Validate())
}

// TestPhase3RoboTorqUnit_JSONSerialization verifies essential fields present, deprecated absent.
func TestPhase3RoboTorqUnit_JSONSerialization(t *testing.T) {
	unit, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10, 2.5, []string{"contract-A"})
	require.NoError(t, err)
	data, err := unit.ToJSON()
	require.NoError(t, err)
	assert.Contains(t, string(data), "unit_id")
	assert.Contains(t, string(data), "merkle_root")
	assert.Contains(t, string(data), "tree_height")
	assert.Contains(t, string(data), "robo_stake_total")
	assert.Contains(t, string(data), "contract_ids")
	assert.Contains(t, string(data), "merkle_proof_api")
	assert.Contains(t, string(data), "minted_at")
	assert.NotContains(t, string(data), "digger_ids")
	assert.NotContains(t, string(data), "refinery_ids")
	// Ensure valid JSON by re-decoding
	var decoded Phase3RoboTorqUnit
	require.NoError(t, json.Unmarshal(data, &decoded))
	assert.Equal(t, unit.MerkleRoot, decoded.MerkleRoot)
}

// TestPhase3RoboTorqUnit_UniqueIDs ensures sequential constructions yield distinct IDs.
func TestPhase3RoboTorqUnit_UniqueIDs(t *testing.T) {
	u1, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10, 1.0, []string{"c"})
	require.NoError(t, err)
	time.Sleep(1 * time.Millisecond)
	u2, err := NewPhase3RoboTorqUnit(generateTestMerkleRoot(), 10, 1.0, []string{"c"})
	require.NoError(t, err)
	assert.NotEqual(t, u1.UnitID, u2.UnitID)
}
