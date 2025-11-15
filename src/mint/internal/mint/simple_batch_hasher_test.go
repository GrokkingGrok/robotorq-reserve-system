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
			JouleTorq:  3600,
			RoboTorq:   100.0,
			Price:      50.0,
			Hash:       "hash1",
			ContractID: "contract1",
			DiggerID:   "digger1",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
		{
			JouleTorq:  3600,
			RoboTorq:   200.0,
			Price:      75.0,
			Hash:       "hash2",
			ContractID: "contract2",
			DiggerID:   "digger2",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	assert.Equal(t, 300.0, totalRobo) // 100 + 200
	assert.Equal(t, 125.0, totalSale) // 50 + 75
	assert.Len(t, batchHash, 64)      // SHA256 hex string is 64 chars
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
		{RoboTorq: 123.45, Price: 10.0, Hash: "h1", Timestamp: time.Now()},
		{RoboTorq: 678.90, Price: 20.0, Hash: "h2", Timestamp: time.Now()},
		{RoboTorq: 234.56, Price: 30.0, Hash: "h3", Timestamp: time.Now()},
	}

	_, totalRobo, _, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.InDelta(t, 1036.91, totalRobo, 0.01) // 123.45 + 678.90 + 234.56
}

func TestSimpleBatchHasher_TotalSaleCalculation(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{RoboTorq: 100.0, Price: 49.99, Hash: "h1", Timestamp: time.Now()},
		{RoboTorq: 100.0, Price: 99.99, Hash: "h2", Timestamp: time.Now()},
		{RoboTorq: 100.0, Price: 149.99, Hash: "h3", Timestamp: time.Now()},
	}

	_, _, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.InDelta(t, 299.97, totalSale, 0.01) // 49.99 + 99.99 + 149.99
}

func TestSimpleBatchHasher_ZeroValues(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{RoboTorq: 0.0, Price: 0.0, Hash: "h1", Timestamp: time.Now()},
		{RoboTorq: 0.0, Price: 0.0, Hash: "h2", Timestamp: time.Now()},
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
			JouleTorq:  3600,
			RoboTorq:   100.0,
			Price:      50.0,
			Hash:       "hash1",
			ContractID: "contract1",
			DiggerID:   "digger1",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
		{
			JouleTorq:  3600,
			RoboTorq:   200.0,
			Price:      75.0,
			Hash:       "hash2",
			ContractID: "contract2",
			DiggerID:   "digger2",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
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
		{RoboTorq: 100.0, Price: 50.0, Hash: "aaa", Timestamp: time.Now()},
		{RoboTorq: 200.0, Price: 75.0, Hash: "bbb", Timestamp: time.Now()},
		{RoboTorq: 300.0, Price: 100.0, Hash: "ccc", Timestamp: time.Now()},
	}

	batch2 := []*TokenTorqIngot{
		{RoboTorq: 300.0, Price: 100.0, Hash: "ccc", Timestamp: time.Now()},
		{RoboTorq: 100.0, Price: 50.0, Hash: "aaa", Timestamp: time.Now()},
		{RoboTorq: 200.0, Price: 75.0, Hash: "bbb", Timestamp: time.Now()},
	}

	hash1, _, _, err1 := hasher.Hash(batch1)
	hash2, _, _, err2 := hasher.Hash(batch2)

	require.NoError(t, err1)
	require.NoError(t, err2)

	// Should produce same hash because hasher sorts by ingot.Hash
	assert.Equal(t, hash1, hash2, "Hash should be order-independent (sorted)")
}

// ─────────────────────────────────────────────────────────────
// Edge Cases
// ─────────────────────────────────────────────────────────────

func TestSimpleBatchHasher_SingleIngot(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	batch := []*TokenTorqIngot{
		{
			JouleTorq: 3600,
			RoboTorq:  500.0,
			Price:     250.0,
			Hash:      "single",
			Timestamp: time.Date(2024, 1, 1, 0, 0, 0, 0, time.UTC),
		},
	}

	batchHash, totalRobo, totalSale, err := hasher.Hash(batch)

	require.NoError(t, err)
	assert.NotEmpty(t, batchHash)
	assert.Equal(t, 500.0, totalRobo)
	assert.Equal(t, 250.0, totalSale)
}

func TestSimpleBatchHasher_LargeBatch(t *testing.T) {
	hasher := NewSimpleBatchHasher()

	// Simulate 1000-ingot batch
	batch := make([]*TokenTorqIngot, 1000)
	expectedRobo := 0.0
	expectedSale := 0.0

	for i := 0; i < 1000; i++ {
		robo := float64(i) * 10.0
		price := float64(i) * 5.0
		batch[i] = &TokenTorqIngot{
			JouleTorq:  3600,
			RoboTorq:   robo,
			Price:      price,
			Hash:       string(rune('a' + (i % 26))),
			ContractID: "contract",
			DiggerID:   "digger",
			Timestamp:  time.Now(),
		}
		expectedRobo += robo
		expectedSale += price
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
		{RoboTorq: 100.0, Price: 50.0, Hash: "hash1", Timestamp: time.Now()},
	}

	batch2 := []*TokenTorqIngot{
		{RoboTorq: 200.0, Price: 50.0, Hash: "hash2", Timestamp: time.Now()},
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
			RoboTorq:  100.0,
			Price:     50.0,
			Hash:      "hash1",
			Timestamp: time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	batch2 := []*TokenTorqIngot{
		{
			RoboTorq:  100.0,
			Price:     50.0,
			Hash:      "hash1",
			Timestamp: time.Date(2024, 1, 2, 12, 0, 0, 0, time.UTC), // Different day
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
			JouleTorq:  3600,
			RoboTorq:   float64(i),
			Price:      float64(i) * 0.5,
			Hash:       string(rune('a' + (i % 26))),
			ContractID: "contract",
			DiggerID:   "digger",
			Timestamp:  time.Now(),
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
