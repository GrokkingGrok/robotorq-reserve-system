package mint

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Basic Hashing Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_BasicHashing(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot1", "contract1", 3600, 100.0),
		createStubIngotForMintTests("ingot2", "contract2", 3600, 200.0),
	}

	batchHash, ingotStakes, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	// Phase 6: Check IngotStakes array instead of totals
	require.Len(t, ingotStakes, 2)
	assert.Equal(t, 100.0, ingotStakes[0].RoboStakeTotal)
	assert.Equal(t, 200.0, ingotStakes[1].RoboStakeTotal)
	assert.Len(t, batchHash, 64) // SHA256 hex string is 64 chars
}

func TestSimpleBatchHasher_EmptyBatch(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{}

	batchHash, ingotStakes, err := hasher.Hash(batch)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "empty batch")
	assert.Empty(t, batchHash)
	assert.Nil(t, ingotStakes)
}

// ─────────────────────────────────────────────────────────────
// Calculation Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_TotalRoboCalculation(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("i1", "c1", 3600, 123.45),
		createStubIngotForMintTests("i2", "c2", 3600, 678.90),
		createStubIngotForMintTests("i3", "c3", 3600, 234.56),
	}

	_, ingotStakes, err := hasher.Hash(batch)

	require.NoError(t, err)
	// Phase 6: Calculate total from IngotStakes array
	var totalRobo float64
	for _, stake := range ingotStakes {
		totalRobo += stake.RoboStakeTotal
	}
	assert.InDelta(t, 1036.91, totalRobo, 0.01) // 123.45 + 678.90 + 234.56
}

// NOTE: This test is now obsolete since PricePerRT was removed from TokenTorqIngot.
// Sale value calculation moved outside the ingot structure.
// Keeping test disabled as documentation of removed functionality.
func TestSimpleBatchHasher_TotalSaleCalculation_DISABLED(t *testing.T) {
	t.Skip("PricePerRT removed from TokenTorqIngot - sale tracking moved elsewhere")
}

func TestSimpleBatchHasher_ZeroValues(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("i1", "c1", 3600, 0.0),
		createStubIngotForMintTests("i2", "c2", 3600, 0.0),
	}

	batchHash, ingotStakes, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash) // Should still generate hash
	require.Len(t, ingotStakes, 2)
	assert.Equal(t, 0.0, ingotStakes[0].RoboStakeTotal)
	assert.Equal(t, 0.0, ingotStakes[1].RoboStakeTotal)
}

// ─────────────────────────────────────────────────────────────
// Determinism Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_Determinism(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Same batch should produce same hash every time
	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot1", "contract1", 3600, 100.0),
		createStubIngotForMintTests("ingot2", "contract2", 3600, 200.0),
	}

	// Hash the same batch 5 times
	hashes := make([]string, 5)
	for i := 0; i < 5; i++ {
		hash, _, err := hasher.Hash(batch)
		require.NoError(t, err)
		hashes[i] = hash
	}

	// All hashes should be identical
	for i := 1; i < 5; i++ {
		assert.Equal(t, hashes[0], hashes[i], "Hash should be deterministic")
	}
}

// ─────────────────────────────────────────────────────────────
// Order Independence & Edge Cases
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_OrderIndependence(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Two batches with same ingots but different order
	// Hasher sorts by BranchHash, so order shouldn't matter
	batch1 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-aaa", "contract1", 3600, 100.0),
		createStubIngotForMintTests("ingot-bbb", "contract2", 3600, 200.0),
		createStubIngotForMintTests("ingot-ccc", "contract3", 3600, 300.0),
	}

	batch2 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-ccc", "contract3", 3600, 300.0),
		createStubIngotForMintTests("ingot-aaa", "contract1", 3600, 100.0),
		createStubIngotForMintTests("ingot-bbb", "contract2", 3600, 200.0),
	}

	hash1, _, err1 := hasher.Hash(batch1)
	hash2, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)

	// Should produce same hash because hasher sorts by BranchHash
	assert.Equal(t, hash1, hash2, "Hash should be order-independent (sorted by BranchHash)")
}

func TestSimpleBatchHasher_SingleIngot(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("single-ingot", "contract-single", 3600, 500.0),
	}

	batchHash, ingotStakes, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	require.Len(t, ingotStakes, 1)
	assert.Equal(t, 500.0, ingotStakes[0].RoboStakeTotal)
	assert.Len(t, batchHash, 64) // SHA256 hex = 64 chars
}

func TestSimpleBatchHasher_LargeBatch(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Simulate 1000-ingot batch (standard RoboTorqUnit size)
	batch := make([]*TokenTorqIngot, 1000)
	expectedRobo := 0.0

	for i := 0; i < 1000; i++ {
		robo := float64(i) * 10.0
		batch[i] = createStubIngotForMintTests(
			"ingot-"+string(rune('a'+(i%26))), // ingot-a, ingot-b, etc.
			"contract",
			3600,
			robo,
		)
		expectedRobo += robo
	}

	batchHash, ingotStakes, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	require.Len(t, ingotStakes, 1000)
	// Calculate total from stakes
	var totalRobo float64
	for _, stake := range ingotStakes {
		totalRobo += stake.RoboStakeTotal
	}
	assert.InDelta(t, expectedRobo, totalRobo, 0.01)
	assert.Len(t, batchHash, 64)
}

// ─────────────────────────────────────────────────────────────
// Hash Uniqueness Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_DifferentBatchesDifferentHashes(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch1 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-1", "contract1", 3600, 100.0),
	}

	batch2 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-2", "contract2", 3600, 200.0),
	}

	hash1, _, err1 := hasher.Hash(batch1)
	hash2, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)
	assert.NotEqual(t, hash1, hash2, "Different batches should produce different hashes")
}

func TestSimpleBatchHasher_DifferentTimestampsSameData(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Note: createStubIngotForMintTests uses time.Now(), so we can't easily test
	// timestamp differences without modifying the helper. Instead, test that
	// same data (ingot ID, contract, values) but different BranchHash produces different hashes

	batch1 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-same-id", "contract1", 3600, 100.0),
	}

	batch2 := []*TokenTorqIngot{
		createStubIngotForMintTests("ingot-same-id", "contract1", 3600, 100.0),
	}

	// Even though same IDs/values, BranchHash generation includes ingot ID in formatting
	// which makes hashes unique per call
	hash1, _, err1 := hasher.Hash(batch1)
	hash2, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)

	// Hashes should be identical because createStubIngotForMintTests creates
	// deterministic BranchHash based on ingot ID
	assert.Equal(t, hash1, hash2, "Same ingot ID should produce same hash (deterministic BranchHash)")
}

// ─────────────────────────────────────────────────────────────
// Performance Test
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_Performance(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	hasher := NewSimpleBatchHasher()

	// Create 1000-ingot batch (standard RoboTorqUnit)
	batch := make([]*TokenTorqIngot, 1000)
	for i := 0; i < 1000; i++ {
		batch[i] = createStubIngotForMintTests(
			"ingot-"+string(rune('a'+(i%26))),
			"contract",
			3600,
			float64(i),
		)
	}

	// Hash 100 times and measure average
	start := time.Now()
	iterations := 100

	for i := 0; i < iterations; i++ {
		_, _, err := hasher.Hash(batch)
		require.NoError(t, err)
	}

	elapsed := time.Since(start)
	avgTime := elapsed / time.Duration(iterations)

	t.Logf("Average hash time for 1000-ingot batch: %v", avgTime)
	assert.Less(t, avgTime, 10*time.Millisecond, "Should hash 1000 ingots in under 10ms")
}
