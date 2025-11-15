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
		{
			IngotID:         "ingot1",
			JouleTorqTotal:  3600,
			RoboStakeTotal:  100.0,
			PricePerRT:      0.5,
			JouleTorqHashes: []string{"hash1"},
			ContractIDs:     []string{"contract1"},
			MintedAt:        time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
		{
			IngotID:         "ingot2",
			JouleTorqTotal:  3600,
			RoboStakeTotal:  200.0,
			PricePerRT:      0.375,
			JouleTorqHashes: []string{"hash2"},
			ContractIDs:     []string{"contract2"},
			MintedAt:        time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	assert.Equal(t, 300.0, totalRobo)         // 100 + 200
	assert.InDelta(t, 125.0, totalSale, 0.01) // (0.5*100) + (0.375*200) = 50 + 75 = 125
	assert.Len(t, batchHash, 64)              // SHA256 hex string is 64 chars
}

func TestSimpleBatchHasher_EmptyBatch(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "empty batch")
	assert.Empty(t, batchHash)
	assert.Equal(t, 0.0, totalRobo)
	assert.Equal(t, 0.0, totalSale)
}

// ─────────────────────────────────────────────────────────────
// Calculation Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_TotalRoboCalculation(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{RoboStakeTotal: 123.45, PricePerRT: 1.0, JouleTorqHashes: []string{"h1"}, MintedAt: time.Now()},
		{RoboStakeTotal: 678.90, PricePerRT: 1.0, JouleTorqHashes: []string{"h2"}, MintedAt: time.Now()},
		{RoboStakeTotal: 234.56, PricePerRT: 1.0, JouleTorqHashes: []string{"h3"}, MintedAt: time.Now()},
	}

	_, totalRobo, _, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.InDelta(t, 1036.91, totalRobo, 0.01) // 123.45 + 678.90 + 234.56
}

func TestSimpleBatchHasher_TotalSaleCalculation(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{RoboStakeTotal: 100.0, PricePerRT: 0.4999, JouleTorqHashes: []string{"h1"}, MintedAt: time.Now()},
		{RoboStakeTotal: 100.0, PricePerRT: 0.9999, JouleTorqHashes: []string{"h2"}, MintedAt: time.Now()},
		{RoboStakeTotal: 100.0, PricePerRT: 1.4999, JouleTorqHashes: []string{"h3"}, MintedAt: time.Now()},
	}

	_, _, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.InDelta(t, 299.97, totalSale, 0.01) // (0.4999*100) + (0.9999*100) + (1.4999*100)
}

func TestSimpleBatchHasher_ZeroValues(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{RoboStakeTotal: 0.0, PricePerRT: 0.0, JouleTorqHashes: []string{"h1"}, MintedAt: time.Now()},
		{RoboStakeTotal: 0.0, PricePerRT: 0.0, JouleTorqHashes: []string{"h2"}, MintedAt: time.Now()},
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash) // Should still generate hash
	assert.Equal(t, 0.0, totalRobo)
	assert.Equal(t, 0.0, totalSale)
}

// ─────────────────────────────────────────────────────────────
// Determinism Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_Determinism(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Same batch should produce same hash every time
	batch := []*TokenTorqIngot{
		{
			IngotID:         "ingot1",
			JouleTorqTotal:  3600,
			RoboStakeTotal:  100.0,
			PricePerRT:      0.5,
			JouleTorqHashes: []string{"hash1"},
			ContractIDs:     []string{"contract1"},
			MintedAt:        time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
		{
			IngotID:         "ingot2",
			JouleTorqTotal:  3600,
			RoboStakeTotal:  200.0,
			PricePerRT:      0.375,
			JouleTorqHashes: []string{"hash2"},
			ContractIDs:     []string{"contract2"},
			MintedAt:        time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	// Hash the same batch 5 times
	hashes := make([]string, 5)
	for i := 0; i < 5; i++ {
		hash, _, _, err := hasher.Hash(batch)
		require.NoError(t, err)
		hashes[i] = hash
	}

	// All hashes should be identical
	for i := 1; i < 5; i++ {
		assert.Equal(t, hashes[0], hashes[i], "Hash should be deterministic")
	}
}

func TestSimpleBatchHasher_OrderIndependence(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Two batches with same ingots but different order
	batch1 := []*TokenTorqIngot{
		{RoboStakeTotal: 100.0, PricePerRT: 0.5, JouleTorqHashes: []string{"aaa"}, MintedAt: time.Now()},
		{RoboStakeTotal: 200.0, PricePerRT: 0.375, JouleTorqHashes: []string{"bbb"}, MintedAt: time.Now()},
		{RoboStakeTotal: 300.0, PricePerRT: 0.333, JouleTorqHashes: []string{"ccc"}, MintedAt: time.Now()},
	}

	batch2 := []*TokenTorqIngot{
		{RoboStakeTotal: 300.0, PricePerRT: 0.333, JouleTorqHashes: []string{"ccc"}, MintedAt: time.Now()},
		{RoboStakeTotal: 100.0, PricePerRT: 0.5, JouleTorqHashes: []string{"aaa"}, MintedAt: time.Now()},
		{RoboStakeTotal: 200.0, PricePerRT: 0.375, JouleTorqHashes: []string{"bbb"}, MintedAt: time.Now()},
	}

	hash1, _, _, err1 := hasher.Hash(batch1)
	hash2, _, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)

	// Should produce same hash because hasher sorts by JouleTorqHashes[0]
	assert.Equal(t, hash1, hash2, "Hash should be order-independent (sorted)")
}

// ─────────────────────────────────────────────────────────────
// Edge Cases
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_SingleIngot(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{
			IngotID:         "single",
			JouleTorqTotal:  3600,
			RoboStakeTotal:  500.0,
			PricePerRT:      0.5,
			JouleTorqHashes: []string{"single"},
			MintedAt:        time.Date(2024, 1, 1, 0, 0, 0, 0, time.UTC),
		},
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	assert.Equal(t, 500.0, totalRobo)
	assert.InDelta(t, 250.0, totalSale, 0.01) // 500 * 0.5
}

func TestSimpleBatchHasher_LargeBatch(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Simulate 1000-ingot batch
	batch := make([]*TokenTorqIngot, 1000)
	expectedRobo := 0.0
	expectedSale := 0.0

	for i := 0; i < 1000; i++ {
		robo := float64(i) * 10.0
		price := 0.5 // Fixed price per RT
		batch[i] = &TokenTorqIngot{
			IngotID:         "ingot-" + string(rune('a'+(i%26))),
			JouleTorqTotal:  3600,
			RoboStakeTotal:  robo,
			PricePerRT:      price,
			JouleTorqHashes: []string{string(rune('a' + (i % 26)))},
			ContractIDs:     []string{"contract"},
			MintedAt:        time.Now(),
		}
		expectedRobo += robo
		expectedSale += price * robo // Sale = PricePerRT * RoboStakeTotal
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	assert.InDelta(t, expectedRobo, totalRobo, 0.01)
	assert.InDelta(t, expectedSale, totalSale, 0.01)
}

// ─────────────────────────────────────────────────────────────
// Hash Uniqueness Tests
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_DifferentBatchesDifferentHashes(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch1 := []*TokenTorqIngot{
		{RoboStakeTotal: 100.0, PricePerRT: 0.5, JouleTorqHashes: []string{"hash1"}, MintedAt: time.Now()},
	}

	batch2 := []*TokenTorqIngot{
		{RoboStakeTotal: 200.0, PricePerRT: 0.5, JouleTorqHashes: []string{"hash2"}, MintedAt: time.Now()},
	}

	hash1, _, _, err1 := hasher.Hash(batch1)
	hash2, _, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)
	assert.NotEqual(t, hash1, hash2, "Different batches should produce different hashes")
}

func TestSimpleBatchHasher_DifferentTimestampsSameData(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Same data but different timestamps
	batch1 := []*TokenTorqIngot{
		{
			RoboStakeTotal:  100.0,
			PricePerRT:      0.5,
			JouleTorqHashes: []string{"hash1"},
			MintedAt:        time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	batch2 := []*TokenTorqIngot{
		{
			RoboStakeTotal:  100.0,
			PricePerRT:      0.5,
			JouleTorqHashes: []string{"hash1"},
			MintedAt:        time.Date(2024, 1, 2, 12, 0, 0, 0, time.UTC), // Different day
		},
	}

	hash1, _, _, err1 := hasher.Hash(batch1)
	hash2, _, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)
	assert.NotEqual(t, hash1, hash2, "Different timestamps should produce different hashes")
}

// ─────────────────────────────────────────────────────────────
// Performance Test
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_Performance(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	hasher := NewSimpleBatchHasher()

	// Create 1000-ingot batch
	batch := make([]*TokenTorqIngot, 1000)
	for i := 0; i < 1000; i++ {
		batch[i] = &TokenTorqIngot{
			IngotID:         "ingot-" + string(rune('a'+(i%26))),
			JouleTorqTotal:  3600,
			RoboStakeTotal:  float64(i),
			PricePerRT:      0.5,
			JouleTorqHashes: []string{string(rune('a' + (i % 26)))},
			ContractIDs:     []string{"contract"},
			MintedAt:        time.Now(),
		}
	}

	// Hash 100 times and measure average
	start := time.Now()
	iterations := 100

	for i := 0; i < iterations; i++ {
		_, _, _, err := hasher.Hash(batch)
		require.NoError(t, err)
	}

	elapsed := time.Since(start)
	avgTime := elapsed / time.Duration(iterations)

	t.Logf("Average hash time for 1000-ingot batch: %v", avgTime)
	assert.Less(t, avgTime, 10*time.Millisecond, "Should hash 1000 ingots in under 10ms")
}
