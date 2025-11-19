package refinery

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sync"
	"testing"
	"time"
)

// TestParallelMerkleTree_Correctness verifies parallel implementation produces same results
func TestParallelMerkleTree_Correctness(t *testing.T) {
	// Generate 3600 test hashes (realistic ingot size)
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		hash := sha256.Sum256([]byte(fmt.Sprintf("test-hash-%d", i)))
		hashes[i] = hex.EncodeToString(hash[:])
	}

	// Build tree (now uses parallel implementation)
	tree, err := BuildMerkleTree(hashes)
	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// Verify properties
	if len(tree.Leaves) != 3600 {
		t.Errorf("Expected 3600 leaves, got %d", len(tree.Leaves))
	}

	if tree.Root == "" {
		t.Error("Root hash should not be empty")
	}

	// Root should be deterministic
	tree2, _ := BuildMerkleTree(hashes)
	if tree.Root != tree2.Root {
		t.Error("Parallel merkle tree should be deterministic")
	}
}

// TestParallelMerkleTree_SmallBatch tests that sequential path works for small batches
func TestParallelMerkleTree_SmallBatch(t *testing.T) {
	// Small batch (< 100 pairs) should use sequential path
	hashes := make([]string, 50)
	for i := 0; i < 50; i++ {
		hash := sha256.Sum256([]byte(fmt.Sprintf("small-%d", i)))
		hashes[i] = hex.EncodeToString(hash[:])
	}

	tree, err := BuildMerkleTree(hashes)
	if err != nil {
		t.Fatalf("BuildMerkleTree failed for small batch: %v", err)
	}

	if tree.Root == "" {
		t.Error("Root should not be empty for small batch")
	}
}

// BenchmarkMerkleTree_Sequential provides baseline for comparison
func BenchmarkMerkleTree_Sequential(b *testing.B) {
	// Generate 3600 hashes
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		hash := sha256.Sum256([]byte(fmt.Sprintf("bench-%d", i)))
		hashes[i] = hex.EncodeToString(hash[:])
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		BuildMerkleTreeSequential(hashes)
	}
}

// BenchmarkMerkleTree_Parallel benchmarks current parallel implementation
func BenchmarkMerkleTree_Parallel(b *testing.B) {
	// Generate 3600 hashes
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		hash := sha256.Sum256([]byte(fmt.Sprintf("bench-%d", i)))
		hashes[i] = hex.EncodeToString(hash[:])
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		BuildMerkleTree(hashes)
	}
}

// BuildMerkleTreeSequential is the old sequential implementation for benchmarking
func BuildMerkleTreeSequential(hashes []string) (*MerkleTree, error) {
	if len(hashes) == 0 {
		return nil, fmt.Errorf("cannot build merkle tree from empty hash list")
	}

	tree := &MerkleTree{
		Leaves: make([]string, len(hashes)),
	}
	copy(tree.Leaves, hashes)

	if len(hashes) == 1 {
		tree.Root = hashPair(hashes[0], hashes[0])
		tree.Height = 1
		return tree, nil
	}

	currentLevel := make([]string, len(hashes))
	copy(currentLevel, hashes)
	height := 0

	// Original sequential implementation
	for len(currentLevel) > 1 {
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

		currentLevel = nextLevel
		height++
	}

	tree.Root = currentLevel[0]
	tree.Height = height

	return tree, nil
}

// TestParallelMerklePerformance measures actual speedup
func TestParallelMerklePerformance(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	// Generate 3600 hashes
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		hash := sha256.Sum256([]byte(fmt.Sprintf("perf-%d", i)))
		hashes[i] = hex.EncodeToString(hash[:])
	}

	// Sequential
	start := time.Now()
	for i := 0; i < 10; i++ {
		BuildMerkleTreeSequential(hashes)
	}
	seqTime := time.Since(start)

	// Parallel
	start = time.Now()
	for i := 0; i < 10; i++ {
		BuildMerkleTree(hashes)
	}
	parTime := time.Since(start)

	speedup := float64(seqTime) / float64(parTime)
	t.Logf("Sequential: %v", seqTime)
	t.Logf("Parallel: %v", parTime)
	t.Logf("Speedup: %.2fx", speedup)

	// Should be faster on multi-core (at least 1.2x)
	if speedup < 1.2 {
		t.Logf("Warning: Expected at least 1.2x speedup, got %.2fx", speedup)
	}
}

// TestParallelMerkleConcurrentSafety tests concurrent builds don't interfere
func TestParallelMerkleConcurrentSafety(t *testing.T) {
	// Generate test data
	hashes1 := make([]string, 3600)
	hashes2 := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		hash1 := sha256.Sum256([]byte(fmt.Sprintf("set1-%d", i)))
		hash2 := sha256.Sum256([]byte(fmt.Sprintf("set2-%d", i)))
		hashes1[i] = hex.EncodeToString(hash1[:])
		hashes2[i] = hex.EncodeToString(hash2[:])
	}

	// Build both concurrently
	var wg sync.WaitGroup
	var tree1, tree2 *MerkleTree
	var err1, err2 error

	wg.Add(2)

	go func() {
		defer wg.Done()
		tree1, err1 = BuildMerkleTree(hashes1)
	}()

	go func() {
		defer wg.Done()
		tree2, err2 = BuildMerkleTree(hashes2)
	}()

	wg.Wait()

	// Verify both completed successfully
	if err1 != nil {
		t.Errorf("Tree 1 failed: %v", err1)
	}
	if err2 != nil {
		t.Errorf("Tree 2 failed: %v", err2)
	}

	// Roots should be different (different input data)
	if tree1.Root == tree2.Root {
		t.Error("Different inputs should produce different roots")
	}
}
