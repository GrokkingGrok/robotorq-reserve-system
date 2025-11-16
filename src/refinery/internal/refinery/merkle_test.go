package refinery

import (
	"crypto/sha256"
	"encoding/hex"
	"testing"
)

// TestBuildMerkleTree_SingleHash tests merkle tree with 1 leaf
func TestBuildMerkleTree_SingleHash(t *testing.T) {
	hash := "a" + string(make([]byte, 63)) // 64 char hash

	tree, err := BuildMerkleTree([]string{hash})

	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// Single hash: root should be hash(hash + hash)
	expectedRoot := hashPair(hash, hash)

	if tree.Root != expectedRoot {
		t.Errorf("Root = %s, want %s", tree.Root, expectedRoot)
	}

	if tree.Height != 1 {
		t.Errorf("Height = %d, want 1", tree.Height)
	}

	if len(tree.Leaves) != 1 {
		t.Errorf("Leaves count = %d, want 1", len(tree.Leaves))
	}
}

// TestBuildMerkleTree_TwoHashes tests merkle tree with 2 leaves (simple pair)
func TestBuildMerkleTree_TwoHashes(t *testing.T) {
	hashA := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	hashB := "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

	tree, err := BuildMerkleTree([]string{hashA, hashB})

	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// Two hashes: root = hash(A + B)
	expectedRoot := hashPair(hashA, hashB)

	if tree.Root != expectedRoot {
		t.Errorf("Root = %s, want %s", tree.Root, expectedRoot)
	}

	if tree.Height != 1 {
		t.Errorf("Height = %d, want 1", tree.Height)
	}
}

// TestBuildMerkleTree_FourHashes tests perfect binary tree (2^2 leaves)
func TestBuildMerkleTree_FourHashes(t *testing.T) {
	hashA := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	hashB := "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
	hashC := "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
	hashD := "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"

	tree, err := BuildMerkleTree([]string{hashA, hashB, hashC, hashD})

	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// Build expected tree manually:
	//       ROOT
	//      /    \
	//    AB      CD
	//   / \     / \
	//  A   B   C   D

	hashAB := hashPair(hashA, hashB)
	hashCD := hashPair(hashC, hashD)
	expectedRoot := hashPair(hashAB, hashCD)

	if tree.Root != expectedRoot {
		t.Errorf("Root = %s, want %s", tree.Root, expectedRoot)
	}

	if tree.Height != 2 {
		t.Errorf("Height = %d, want 2", tree.Height)
	}
}

// TestBuildMerkleTree_OddNumber tests tree with 3 leaves (duplicates last)
func TestBuildMerkleTree_OddNumber(t *testing.T) {
	hashA := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	hashB := "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
	hashC := "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

	tree, err := BuildMerkleTree([]string{hashA, hashB, hashC})

	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// Build expected tree (C gets duplicated):
	//       ROOT
	//      /    \
	//    AB      CC
	//   / \     / \
	//  A   B   C   C (duplicated)

	hashAB := hashPair(hashA, hashB)
	hashCC := hashPair(hashC, hashC) // Duplicate last hash
	expectedRoot := hashPair(hashAB, hashCC)

	if tree.Root != expectedRoot {
		t.Errorf("Root = %s, want %s", tree.Root, expectedRoot)
	}

	if tree.Height != 2 {
		t.Errorf("Height = %d, want 2", tree.Height)
	}
}

// TestBuildMerkleTree_3600Hashes tests with ingot-sized hash count
func TestBuildMerkleTree_3600Hashes(t *testing.T) {
	// Generate 3600 unique hashes
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		// Create unique hash from index
		data := []byte{byte(i >> 8), byte(i & 0xFF)}
		hash := sha256.Sum256(data)
		hashes[i] = hex.EncodeToString(hash[:])
	}

	tree, err := BuildMerkleTree(hashes)

	if err != nil {
		t.Fatalf("BuildMerkleTree failed: %v", err)
	}

	// 3600 leaves: height = ceil(log₂(3600)) = 12
	// 2^11 = 2048 < 3600 < 4096 = 2^12
	expectedHeight := 12

	if tree.Height != expectedHeight {
		t.Errorf("Height = %d, want %d", tree.Height, expectedHeight)
	}

	if len(tree.Root) != 64 {
		t.Errorf("Root hash length = %d, want 64 (SHA256 hex)", len(tree.Root))
	}

	if len(tree.Leaves) != 3600 {
		t.Errorf("Leaves count = %d, want 3600", len(tree.Leaves))
	}
}

// TestBuildMerkleTree_Deterministic tests same inputs produce same root
func TestBuildMerkleTree_Deterministic(t *testing.T) {
	hashes := []string{
		"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
	}

	tree1, err := BuildMerkleTree(hashes)
	if err != nil {
		t.Fatalf("BuildMerkleTree #1 failed: %v", err)
	}

	tree2, err := BuildMerkleTree(hashes)
	if err != nil {
		t.Fatalf("BuildMerkleTree #2 failed: %v", err)
	}

	if tree1.Root != tree2.Root {
		t.Errorf("Determinism check failed: root1 = %s, root2 = %s", tree1.Root, tree2.Root)
	}
}

// TestBuildMerkleTree_OrderMatters tests different order produces different root
func TestBuildMerkleTree_OrderMatters(t *testing.T) {
	hashesABC := []string{
		"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
	}

	hashesCBA := []string{
		"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
		"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
	}

	treeABC, _ := BuildMerkleTree(hashesABC)
	treeCBA, _ := BuildMerkleTree(hashesCBA)

	if treeABC.Root == treeCBA.Root {
		t.Errorf("Order should matter: ABC and CBA produced same root")
	}
}

// TestBuildMerkleTree_EmptyInput tests error handling for empty hash list
func TestBuildMerkleTree_EmptyInput(t *testing.T) {
	_, err := BuildMerkleTree([]string{})

	if err == nil {
		t.Fatal("Expected error for empty hash list, got nil")
	}

	expectedMsg := "cannot build merkle tree from empty hash list"
	if err.Error() != expectedMsg {
		t.Errorf("Error message = %q, want %q", err.Error(), expectedMsg)
	}
}

// TestVerifyRoot tests root verification function
func TestVerifyRoot(t *testing.T) {
	hashes := []string{
		"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
	}

	tree, _ := BuildMerkleTree(hashes)

	// Verify with correct root
	valid, err := VerifyRoot(hashes, tree.Root)
	if err != nil {
		t.Fatalf("VerifyRoot failed: %v", err)
	}
	if !valid {
		t.Error("VerifyRoot returned false for correct root")
	}

	// Verify with wrong root
	wrongRoot := "0000000000000000000000000000000000000000000000000000000000000000"
	valid, err = VerifyRoot(hashes, wrongRoot)
	if err != nil {
		t.Fatalf("VerifyRoot failed: %v", err)
	}
	if valid {
		t.Error("VerifyRoot returned true for incorrect root")
	}
}

// TestHashPair tests the hash pairing function
func TestHashPair(t *testing.T) {
	hashA := "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
	hashB := "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

	result := hashPair(hashA, hashB)

	// Should be valid SHA256 hex (64 chars)
	if len(result) != 64 {
		t.Errorf("hashPair result length = %d, want 64", len(result))
	}

	// Should be deterministic
	result2 := hashPair(hashA, hashB)
	if result != result2 {
		t.Error("hashPair not deterministic")
	}

	// Different order should produce different hash
	resultBA := hashPair(hashB, hashA)
	if result == resultBA {
		t.Error("hashPair(A,B) should differ from hashPair(B,A)")
	}
}

// BenchmarkBuildMerkleTree_3600 benchmarks merkle tree construction
func BenchmarkBuildMerkleTree_3600(b *testing.B) {
	// Generate 3600 hashes once
	hashes := make([]string, 3600)
	for i := 0; i < 3600; i++ {
		data := []byte{byte(i >> 8), byte(i & 0xFF)}
		hash := sha256.Sum256(data)
		hashes[i] = hex.EncodeToString(hash[:])
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		BuildMerkleTree(hashes)
	}
}
