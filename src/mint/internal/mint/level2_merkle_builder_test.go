package mint

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewLevel2MerkleBuilder tests builder creation
func TestNewLevel2MerkleBuilder(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)

	assert.NotNil(t, builder)
	assert.NotNil(t, builder.queue)
	assert.NotNil(t, builder.logger)
	assert.NotNil(t, builder.metrics)
}

// TestBuildLevel2Tree_Success tests successful tree building
func TestBuildLevel2Tree_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries in background
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{fmt.Sprintf("contract-%d", i%10)},
				DiggerIDs:   []string{fmt.Sprintf("digger-%d", i%5)},
				RefineryID:  fmt.Sprintf("refinery-%d", i%3),
			}
			err := queue.AddIngotHash(ctx, entry)
			require.NoError(t, err)
		}
	}()

	// Build tree (blocks until 1000 hashes ready)
	result, err := builder.BuildLevel2Tree(ctx)

	require.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.MerkleRoot, 64, "merkle root should be 64-char hex")
	assert.Len(t, result.HashEntries, 1000)
	assert.Equal(t, 10, result.TreeHeight, "tree height for 1000 hashes should be 10")
	assert.Len(t, result.ContractIDs, 10, "should have 10 unique contracts")
	assert.Len(t, result.DiggerIDs, 5, "should have 5 unique diggers")
	assert.Len(t, result.RefineryIDs, 3, "should have 3 unique refineries")
}

// TestBuildLevel2Tree_Deterministic tests that same hashes produce same root
func TestBuildLevel2Tree_Deterministic(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))

	// Build tree 1
	queue1, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	builder1 := NewLevel2MerkleBuilder(queue1, logger)
	ctx := context.Background()

	// Add same hashes to queue 1
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue1.AddIngotHash(ctx, entry)
		}
	}()

	result1, err := builder1.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Build tree 2
	queue2, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	builder2 := NewLevel2MerkleBuilder(queue2, logger)

	// Add same hashes to queue 2
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue2.AddIngotHash(ctx, entry)
		}
	}()

	result2, err := builder2.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Verify determinism: same inputs → same root
	assert.Equal(t, result1.MerkleRoot, result2.MerkleRoot,
		"same hashes should produce same merkle root")
	assert.Equal(t, result1.TreeHeight, result2.TreeHeight)
}

// TestBuildLevel2Tree_ContextCancellation tests graceful cancellation
func TestBuildLevel2Tree_ContextCancellation(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)

	// Create cancellable context
	ctx, cancel := context.WithCancel(context.Background())

	// Cancel immediately (before 1000 hashes available)
	cancel()

	// Should return error due to cancellation
	result, err := builder.BuildLevel2Tree(ctx)

	assert.Error(t, err)
	assert.Nil(t, result)
	assert.Contains(t, err.Error(), "context cancelled")
}

// TestBuildLevel2Tree_InvalidHashLength tests invalid hash validation
func TestBuildLevel2Tree_InvalidHashLength(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 999 valid hashes + 1 invalid hash
	go func() {
		for i := 0; i < 999; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}

		// Add invalid hash (too short)
		invalidEntry := &models.IngotHashEntry{
			BranchHash:  "short",
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			RefineryID:  "refinery-1",
		}
		_ = queue.AddIngotHash(ctx, invalidEntry)
	}()

	// Should fail validation
	result, err := builder.BuildLevel2Tree(ctx)

	assert.Error(t, err)
	assert.Nil(t, result)
	assert.Contains(t, err.Error(), "invalid branch_hash length")
}

// TestBuildMerkleTree_SingleHash tests tree building with 1 hash
func TestBuildMerkleTree_SingleHash(t *testing.T) {
	hashes := []string{generateTestHash(0)}

	root, height, treeNodes, err := buildMerkleTree(hashes)

	require.NoError(t, err)
	assert.Len(t, root, 64, "root should be 64-char hex")
	assert.Equal(t, 1, height, "height should be 1 for single hash (1 level up from leaf)")
	assert.NotEmpty(t, treeNodes, "tree nodes should be populated")
	assert.Len(t, treeNodes, 2, "should have 2 levels (leaf + root)")
}

// TestBuildMerkleTree_TwoHashes tests tree building with 2 hashes
func TestBuildMerkleTree_TwoHashes(t *testing.T) {
	hashes := []string{
		generateTestHash(0),
		generateTestHash(1),
	}

	root, height, treeNodes, err := buildMerkleTree(hashes)

	require.NoError(t, err)
	assert.Len(t, root, 64)
	assert.Equal(t, 1, height, "height should be 1 for 2 hashes (1 level up)")
	assert.Len(t, treeNodes, 2, "should have 2 levels (leaves + root)")
}

// TestBuildMerkleTree_OddNumber tests tree building with odd number of hashes
func TestBuildMerkleTree_OddNumber(t *testing.T) {
	hashes := []string{
		generateTestHash(0),
		generateTestHash(1),
		generateTestHash(2),
	}

	root, height, treeNodes, err := buildMerkleTree(hashes)

	require.NoError(t, err)
	assert.Len(t, root, 64)
	assert.Equal(t, 2, height, "height should be 2 for 3 hashes (2 levels up)")
	assert.Len(t, treeNodes, 3, "should have 3 levels (leaves + 2 intermediate)")
}

// TestBuildMerkleTree_PowerOfTwo tests tree building with power of 2 hashes
func TestBuildMerkleTree_PowerOfTwo(t *testing.T) {
	// 16 hashes = 2^4
	hashes := make([]string, 16)
	for i := 0; i < 16; i++ {
		hashes[i] = generateTestHash(i)
	}

	root, height, treeNodes, err := buildMerkleTree(hashes)

	require.NoError(t, err)
	assert.Len(t, root, 64)
	assert.Equal(t, 4, height, "height should be 4 for 16 hashes (log₂(16))")
	assert.Len(t, treeNodes, 5, "should have 5 levels (leaf + 4 up to root)")
}

// TestBuildMerkleTree_1000Hashes tests tree building with 1000 hashes (real scenario)
func TestBuildMerkleTree_1000Hashes(t *testing.T) {
	hashes := make([]string, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = generateTestHash(i)
	}

	start := time.Now()
	root, height, treeNodes, err := buildMerkleTree(hashes)
	duration := time.Since(start)

	require.NoError(t, err)
	assert.Len(t, root, 64)
	assert.Equal(t, 10, height, "height should be 10 for 1000 hashes (⌈log₂(1000)⌉)")
	assert.Len(t, treeNodes, 11, "should have 11 levels (leaf + 10 up to root)")
	assert.Less(t, duration.Milliseconds(), int64(20), "should build tree in <20ms")

	t.Logf("Built merkle tree from 1000 hashes in %v", duration)
	t.Logf("Merkle root: %s", root)
}

// TestBuildMerkleTree_EmptyList tests error handling for empty hash list
func TestBuildMerkleTree_EmptyList(t *testing.T) {
	hashes := []string{}

	root, height, treeNodes, err := buildMerkleTree(hashes)

	assert.Error(t, err)
	assert.Empty(t, root)
	assert.Equal(t, 0, height)
	assert.Nil(t, treeNodes)
	assert.Contains(t, err.Error(), "empty hash list")
}

// TestHashPair tests SHA256 hash pair combination
func TestHashPair(t *testing.T) {
	left := generateTestHash(0)
	right := generateTestHash(1)

	result := hashPair(left, right)

	assert.Len(t, result, 64, "hash should be 64-char hex")

	// Verify determinism
	result2 := hashPair(left, right)
	assert.Equal(t, result, result2, "same inputs should produce same hash")

	// Verify different inputs produce different hashes
	result3 := hashPair(left, generateTestHash(2))
	assert.NotEqual(t, result, result3, "different inputs should produce different hashes")
}

// TestExtractUniqueIDs tests unique ID extraction
func TestExtractUniqueIDs(t *testing.T) {
	entries := []*models.IngotHashEntry{
		{ContractIDs: []string{"contract-1", "contract-2"}},
		{ContractIDs: []string{"contract-2", "contract-3"}},
		{ContractIDs: []string{"contract-1", "contract-4"}},
	}

	unique := extractUniqueIDs(entries, func(e *models.IngotHashEntry) []string { return e.ContractIDs })

	assert.Len(t, unique, 4, "should have 4 unique contracts")
	assert.Contains(t, unique, "contract-1")
	assert.Contains(t, unique, "contract-2")
	assert.Contains(t, unique, "contract-3")
	assert.Contains(t, unique, "contract-4")
}

// TestExtractUniqueStrings tests unique string extraction
func TestExtractUniqueStrings(t *testing.T) {
	entries := []*models.IngotHashEntry{
		{RefineryID: "refinery-1"},
		{RefineryID: "refinery-2"},
		{RefineryID: "refinery-1"},
		{RefineryID: "refinery-3"},
	}

	unique := extractUniqueStrings(entries, func(e *models.IngotHashEntry) string { return e.RefineryID })

	assert.Len(t, unique, 3, "should have 3 unique refineries")
	assert.Contains(t, unique, "refinery-1")
	assert.Contains(t, unique, "refinery-2")
	assert.Contains(t, unique, "refinery-3")
}

// generateTestHash generates a deterministic 64-char hex hash from index
func generateTestHash(index int) string {
	return fmt.Sprintf("%064d", index)
}

// =======================
// Merkle Proof Tests
// =======================

// TestLevel2MerkleResult_GetProof_ValidIndex tests proof generation for valid index
func TestLevel2MerkleResult_GetProof_ValidIndex(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}
	}()

	result, err := builder.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Get proof for various indices
	testCases := []int{0, 1, 42, 500, 999}

	for _, index := range testCases {
		proof, err := result.GetProof(index)
		require.NoError(t, err, "should get proof for index %d", index)
		assert.NotNil(t, proof)
		assert.Equal(t, result.TreeHeight, len(proof), "proof length should equal tree height")
	}
}

// TestLevel2MerkleResult_GetProof_InvalidIndex tests proof generation for invalid indices
func TestLevel2MerkleResult_GetProof_InvalidIndex(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}
	}()

	result, err := builder.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Test invalid indices
	testCases := []struct {
		index       int
		description string
	}{
		{-1, "negative index"},
		{1000, "index equal to count"},
		{5000, "index far too large"},
	}

	for _, tc := range testCases {
		proof, err := result.GetProof(tc.index)
		assert.Error(t, err, "should error for %s", tc.description)
		assert.Nil(t, proof, "proof should be nil for %s", tc.description)
		assert.Contains(t, err.Error(), "out of bounds", "error message should mention bounds")
	}
}

// TestLevel2MerkleResult_GetProof_NoTreeNodes tests error when TreeNodes not available
func TestLevel2MerkleResult_GetProof_NoTreeNodes(t *testing.T) {
	result := &Level2MerkleResult{
		MerkleRoot:  generateTestHash(0),
		HashEntries: make([]*models.IngotHashEntry, 1000),
		TreeHeight:  10,
		TreeNodes:   nil, // No tree nodes!
	}

	proof, err := result.GetProof(0)
	assert.Error(t, err)
	assert.Nil(t, proof)
	assert.Contains(t, err.Error(), "tree nodes not available")
}

// TestVerifyProof_ValidProof tests verification of valid proofs
func TestVerifyProof_ValidProof(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}
	}()

	result, err := builder.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Verify proofs for all 1000 ingots
	for i := 0; i < 1000; i++ {
		leafHash := result.HashEntries[i].BranchHash
		proof, err := result.GetProof(i)
		require.NoError(t, err)

		// Verify proof
		valid := VerifyProof(leafHash, proof, result.MerkleRoot, i)
		assert.True(t, valid, "proof should be valid for index %d", i)
	}
}

// TestVerifyProof_InvalidProof tests verification of invalid proofs
func TestVerifyProof_InvalidProof(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}
	}()

	result, err := builder.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Test 1: Wrong leaf hash
	leafHash := result.HashEntries[42].BranchHash
	proof, _ := result.GetProof(42)
	wrongLeafHash := generateTestHash(99999)
	valid := VerifyProof(wrongLeafHash, proof, result.MerkleRoot, 42)
	assert.False(t, valid, "proof should be invalid with wrong leaf hash")

	// Test 2: Wrong merkle root
	wrongRoot := generateTestHash(88888)
	valid = VerifyProof(leafHash, proof, wrongRoot, 42)
	assert.False(t, valid, "proof should be invalid with wrong root")

	// Test 3: Tampered proof (flip one hash)
	tamperedProof := make([]string, len(proof))
	copy(tamperedProof, proof)
	tamperedProof[0] = generateTestHash(77777)
	valid = VerifyProof(leafHash, tamperedProof, result.MerkleRoot, 42)
	assert.False(t, valid, "proof should be invalid with tampered proof")
}

// TestVerifyProof_LargeTree tests verification on large tree (1000 leaves)
func TestVerifyProof_LargeTree(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	builder := NewLevel2MerkleBuilder(queue, logger)
	ctx := context.Background()

	// Add 1000 hash entries
	go func() {
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  generateTestHash(i),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				RefineryID:  "refinery-1",
			}
			_ = queue.AddIngotHash(ctx, entry)
		}
	}()

	result, err := builder.BuildLevel2Tree(ctx)
	require.NoError(t, err)

	// Spot check random indices
	testIndices := []int{0, 1, 100, 250, 500, 750, 999}

	for _, index := range testIndices {
		leafHash := result.HashEntries[index].BranchHash
		proof, err := result.GetProof(index)
		require.NoError(t, err)

		valid := VerifyProof(leafHash, proof, result.MerkleRoot, index)
		assert.True(t, valid, "proof should be valid for index %d in large tree", index)
	}
}

// TestRoundTrip_BuildProveVerify tests complete round-trip for various tree sizes
func TestRoundTrip_BuildProveVerify(t *testing.T) {
	testCases := []struct {
		count       int
		description string
	}{
		{4, "small tree (4 ingots)"},
		{100, "medium tree (100 ingots)"},
		{1000, "full tree (1000 ingots)"},
	}

	for _, tc := range testCases {
		t.Run(tc.description, func(t *testing.T) {
			logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
			queue, err := NewIngotHashQueue(tc.count+100, tc.count, logger)
			require.NoError(t, err)

			builder := NewLevel2MerkleBuilder(queue, logger)
			ctx := context.Background()

			// Add hash entries
			go func() {
				for i := 0; i < tc.count; i++ {
					entry := &models.IngotHashEntry{
						BranchHash:  generateTestHash(i),
						ContractIDs: []string{fmt.Sprintf("contract-%d", i%5)},
						DiggerIDs:   []string{fmt.Sprintf("digger-%d", i%3)},
						RefineryID:  fmt.Sprintf("refinery-%d", i%2),
					}
					_ = queue.AddIngotHash(ctx, entry)
				}
			}()

			// Build tree
			result, err := builder.BuildLevel2Tree(ctx)
			require.NoError(t, err)

			// Verify proofs for all entries
			for i := 0; i < tc.count; i++ {
				leafHash := result.HashEntries[i].BranchHash
				proof, err := result.GetProof(i)
				require.NoError(t, err, "should get proof for index %d", i)

				valid := VerifyProof(leafHash, proof, result.MerkleRoot, i)
				assert.True(t, valid, "round-trip should verify for index %d in %s", i, tc.description)
			}
		})
	}
}

// TestProofSize_Logarithmic tests that proof size is logarithmic
func TestProofSize_Logarithmic(t *testing.T) {
	testCases := []struct {
		count                int
		expectedMaxProofSize int
		description          string
	}{
		{4, 2, "4 leaves -> log₂(4) = 2"},
		{16, 4, "16 leaves -> log₂(16) = 4"},
		{100, 7, "100 leaves -> ⌈log₂(100)⌉ = 7"},
		{1000, 10, "1000 leaves -> ⌈log₂(1000)⌉ = 10"},
	}

	for _, tc := range testCases {
		t.Run(tc.description, func(t *testing.T) {
			logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
			queue, err := NewIngotHashQueue(tc.count+100, tc.count, logger)
			require.NoError(t, err)

			builder := NewLevel2MerkleBuilder(queue, logger)
			ctx := context.Background()

			// Add hash entries
			go func() {
				for i := 0; i < tc.count; i++ {
					entry := &models.IngotHashEntry{
						BranchHash:  generateTestHash(i),
						ContractIDs: []string{"contract-1"},
						DiggerIDs:   []string{"digger-1"},
						RefineryID:  "refinery-1",
					}
					_ = queue.AddIngotHash(ctx, entry)
				}
			}()

			result, err := builder.BuildLevel2Tree(ctx)
			require.NoError(t, err)

			// Get proof for middle element
			proof, err := result.GetProof(tc.count / 2)
			require.NoError(t, err)

			// Verify proof size is logarithmic
			assert.LessOrEqual(t, len(proof), tc.expectedMaxProofSize,
				"proof size should be ≤ %d for %s", tc.expectedMaxProofSize, tc.description)

			t.Logf("%s: proof size = %d hashes (expected ≤ %d)",
				tc.description, len(proof), tc.expectedMaxProofSize)
		})
	}
}
