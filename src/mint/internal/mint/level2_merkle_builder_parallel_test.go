package mint

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestParallelMerkleTree_Correctness verifies the parallel implementation
// produces identical results to sequential for 1000 ingot hashes
func TestParallelMerkleTree_Correctness(t *testing.T) {
	// Generate 1000 test hashes (simulating 1000 ingots)
	hashes := make([]string, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = fmt.Sprintf("hash%04d", i)
	}

	// Build tree with parallel implementation
	root1, height1, levels1, err1 := buildMerkleTree(hashes)
	require.NoError(t, err1)

	// Build tree with sequential implementation
	root2, height2, levels2, err2 := buildMerkleTreeSequential(hashes)
	require.NoError(t, err2)

	// Verify identical results
	assert.Equal(t, root2, root1, "Root hash should match sequential")
	assert.Equal(t, height2, height1, "Tree height should match sequential")
	assert.Equal(t, len(levels2), len(levels1), "Number of levels should match")

	// Verify each level matches
	for i := 0; i < len(levels1); i++ {
		assert.Equal(t, len(levels2[i]), len(levels1[i]), "Level %d size should match", i)
		assert.ElementsMatch(t, levels2[i], levels1[i], "Level %d hashes should match", i)
	}

	// Verify tree structure for 1000 hashes
	// Level 0: 1000 leaves
	// Level 1: 500 pairs
	// Level 2: 250 pairs
	// Level 3: 125 pairs
	// Level 4: 63 pairs (125 is odd, so 62 pairs + 1 duplicated)
	// ... continues to root
	assert.Equal(t, 1000, len(levels1[0]), "Level 0 should have 1000 leaves")
	assert.Equal(t, 500, len(levels1[1]), "Level 1 should have 500 hashes")
	assert.Equal(t, 250, len(levels1[2]), "Level 2 should have 250 hashes")
	assert.Equal(t, 125, len(levels1[3]), "Level 3 should have 125 hashes")
	assert.Equal(t, 63, len(levels1[4]), "Level 4 should have 63 hashes")
}

// TestParallelMerkleTree_SmallBatch verifies sequential path is used for <100 pairs
func TestParallelMerkleTree_SmallBatch(t *testing.T) {
	// Generate 64 hashes (32 pairs at first level - below threshold)
	hashes := make([]string, 64)
	for i := 0; i < 64; i++ {
		hashes[i] = fmt.Sprintf("small%04d", i)
	}

	root, height, levels, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	// Verify correctness
	assert.NotEmpty(t, root)
	assert.Equal(t, 64, len(levels[0]))
	assert.Equal(t, 32, len(levels[1]), "First level should have 32 pairs")
	assert.Equal(t, 16, len(levels[2]), "Second level should have 16 pairs")

	// Verify matches sequential
	rootSeq, heightSeq, levelsSeq, errSeq := buildMerkleTreeSequential(hashes)
	require.NoError(t, errSeq)
	assert.Equal(t, rootSeq, root)
	assert.Equal(t, heightSeq, height)
	assert.Equal(t, len(levelsSeq), len(levels))
}

// TestParallelMerkleTree_Performance measures speedup of parallel implementation
func TestParallelMerkleTree_Performance(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	// Generate 1000 hashes
	hashes := make([]string, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = fmt.Sprintf("perf%04d", i)
	}

	// Warm up
	_, _, _, _ = buildMerkleTree(hashes)
	_, _, _, _ = buildMerkleTreeSequential(hashes)

	// Measure parallel
	startParallel := time.Now()
	for i := 0; i < 100; i++ {
		_, _, _, _ = buildMerkleTree(hashes)
	}
	parallelDuration := time.Since(startParallel)

	// Measure sequential
	startSequential := time.Now()
	for i := 0; i < 100; i++ {
		_, _, _, _ = buildMerkleTreeSequential(hashes)
	}
	sequentialDuration := time.Since(startSequential)

	speedup := float64(sequentialDuration) / float64(parallelDuration)
	t.Logf("Performance comparison (100 iterations, 1000 hashes):")
	t.Logf("  Parallel:   %v", parallelDuration)
	t.Logf("  Sequential: %v", sequentialDuration)
	t.Logf("  Speedup:    %.2fx", speedup)

	// On multi-core systems, expect some speedup (>1.0x)
	// Don't enforce strict threshold as it depends on hardware
	assert.Greater(t, speedup, 0.8, "Parallel should not be significantly slower than sequential")
}

// TestParallelMerkleTree_ConcurrentSafety verifies concurrent tree builds don't interfere
func TestParallelMerkleTree_ConcurrentSafety(t *testing.T) {
	// Generate different hash sets
	hashSets := make([][]string, 10)
	for i := 0; i < 10; i++ {
		hashSets[i] = make([]string, 1000)
		for j := 0; j < 1000; j++ {
			hashSets[i][j] = fmt.Sprintf("set%d-hash%04d", i, j)
		}
	}

	// Build all trees concurrently
	var wg sync.WaitGroup
	results := make([]string, 10)

	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			root, _, _, err := buildMerkleTree(hashSets[idx])
			require.NoError(t, err)
			results[idx] = root
		}(i)
	}

	wg.Wait()

	// Verify all roots are different (different input sets)
	seen := make(map[string]bool)
	for _, root := range results {
		assert.False(t, seen[root], "Each hash set should produce unique root")
		seen[root] = true
	}

	// Verify results match sequential builds
	for i := 0; i < 10; i++ {
		rootSeq, _, _, err := buildMerkleTreeSequential(hashSets[i])
		require.NoError(t, err)
		assert.Equal(t, rootSeq, results[i], "Concurrent build %d should match sequential", i)
	}
}

// TestParallelMerkleTree_OddNumberOfHashes verifies handling of odd-numbered levels
func TestParallelMerkleTree_OddNumberOfHashes(t *testing.T) {
	// Test with 999 hashes (odd number)
	hashes := make([]string, 999)
	for i := 0; i < 999; i++ {
		hashes[i] = fmt.Sprintf("odd%04d", i)
	}

	root, height, levels, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	// Verify correctness
	assert.NotEmpty(t, root)
	assert.Equal(t, 999, len(levels[0]))
	assert.Equal(t, 500, len(levels[1]), "Level 1 should have 500 (999 pairs, last duplicated)")

	// Verify matches sequential
	rootSeq, heightSeq, levelsSeq, errSeq := buildMerkleTreeSequential(hashes)
	require.NoError(t, errSeq)
	assert.Equal(t, rootSeq, root)
	assert.Equal(t, heightSeq, height)
	assert.Equal(t, len(levelsSeq), len(levels))
}

// BenchmarkMerkleTree_Parallel benchmarks the parallel implementation
func BenchmarkMerkleTree_Parallel(b *testing.B) {
	hashes := make([]string, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = fmt.Sprintf("bench%04d", i)
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _, _, _ = buildMerkleTree(hashes)
	}
}

// BenchmarkMerkleTree_Sequential benchmarks the sequential implementation
func BenchmarkMerkleTree_Sequential(b *testing.B) {
	hashes := make([]string, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = fmt.Sprintf("bench%04d", i)
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _, _, _ = buildMerkleTreeSequential(hashes)
	}
}

// buildMerkleTreeSequential is the original sequential implementation
// preserved for testing and benchmarking comparison
func buildMerkleTreeSequential(hashes []string) (string, int, [][]string, error) {
	if len(hashes) == 0 {
		return "", 0, nil, fmt.Errorf("cannot build merkle tree: no hashes provided")
	}

	if len(hashes) == 1 {
		return hashes[0], 0, [][]string{hashes}, nil
	}

	var allLevels [][]string
	currentLevel := make([]string, len(hashes))
	copy(currentLevel, hashes)
	allLevels = append(allLevels, currentLevel)

	height := 0

	// Sequential implementation
	for len(currentLevel) > 1 {
		height++
		nextLevel := make([]string, 0, (len(currentLevel)+1)/2)

		for i := 0; i < len(currentLevel); i += 2 {
			var combinedHash string

			if i+1 < len(currentLevel) {
				combinedHash = hashPair(currentLevel[i], currentLevel[i+1])
			} else {
				combinedHash = hashPair(currentLevel[i], currentLevel[i])
			}

			nextLevel = append(nextLevel, combinedHash)
		}

		allLevels = append(allLevels, nextLevel)
		currentLevel = nextLevel
	}

	return currentLevel[0], height, allLevels, nil
}
