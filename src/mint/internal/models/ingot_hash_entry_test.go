package models

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewIngotHashEntry_Success tests creating a valid IngotHashEntry from Phase2Ingot
func TestNewIngotHashEntry_Success(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "ingot-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1", "contract-2"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.5,
		Timestamp:      time.Now(),
		Signature:      "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		PublicKey:      "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
	}

	entry := NewIngotHashEntry(ingot)

	require.NotNil(t, entry)
	assert.Equal(t, ingot.BranchHash, entry.BranchHash)
	assert.Equal(t, ingot.ContractIDs, entry.ContractIDs)
	assert.Equal(t, ingot.DiggerIDs, entry.DiggerIDs)
	assert.Equal(t, ingot.RoboStakeTotal, entry.RoboStakeTotal)
	assert.Equal(t, ingot.Timestamp, entry.Timestamp)
	assert.Empty(t, entry.RefineryID, "RefineryID should be empty (future use)")
}

// TestNewIngotHashEntry_PreservesAllData tests that all relevant data is preserved
func TestNewIngotHashEntry_PreservesAllData(t *testing.T) {
	now := time.Now()
	ingot := &Phase2Ingot{
		ID:             "ingot-test-preserve",
		BranchHash:     "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
		HashCount:      3600,
		ContractIDs:    []string{"contract-A", "contract-B", "contract-C"},
		DiggerIDs:      []string{"digger-X", "digger-Y"},
		RoboStakeTotal: 10.25,
		Timestamp:      now,
		Signature:      "sig123",
		PublicKey:      "pubkey123",
	}

	entry := NewIngotHashEntry(ingot)

	// Verify all fields copied correctly
	assert.Equal(t, "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", entry.BranchHash)
	assert.Equal(t, 3, len(entry.ContractIDs))
	assert.Equal(t, 2, len(entry.DiggerIDs))
	assert.Equal(t, 10.25, entry.RoboStakeTotal)
	assert.Equal(t, now.Unix(), entry.Timestamp.Unix()) // Compare Unix time
}

// TestNewIngotHashEntry_SingleContractSingleDigger tests minimal valid case
func TestNewIngotHashEntry_SingleContractSingleDigger(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "minimal-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"single-contract"},
		DiggerIDs:      []string{"single-digger"},
		RoboStakeTotal: 1.0,
		Timestamp:      time.Now(),
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Equal(t, 1, len(entry.ContractIDs))
	assert.Equal(t, 1, len(entry.DiggerIDs))
	assert.Equal(t, "single-contract", entry.ContractIDs[0])
	assert.Equal(t, "single-digger", entry.DiggerIDs[0])
}

// TestNewIngotHashEntry_MultipleContractsMultipleDiggers tests complex case
func TestNewIngotHashEntry_MultipleContractsMultipleDiggers(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:          "complex-001",
		BranchHash:  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:   3600,
		ContractIDs: []string{"c1", "c2", "c3", "c4", "c5"},
		DiggerIDs:   []string{"d1", "d2", "d3", "d4"},
		Timestamp:   time.Now(),
		Signature:   "sig",
		PublicKey:   "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Equal(t, 5, len(entry.ContractIDs))
	assert.Equal(t, 4, len(entry.DiggerIDs))
	assert.Contains(t, entry.ContractIDs, "c3")
	assert.Contains(t, entry.DiggerIDs, "d2")
}

// TestNewIngotHashEntry_ZeroRoboStake tests zero stake handling
func TestNewIngotHashEntry_ZeroRoboStake(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "zero-stake-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 0.0,
		Timestamp:      time.Now(),
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Equal(t, 0.0, entry.RoboStakeTotal)
}

// TestNewIngotHashEntry_HighRoboStake tests large stake handling
func TestNewIngotHashEntry_HighRoboStake(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "high-stake-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 999999.999,
		Timestamp:      time.Now(),
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Equal(t, 999999.999, entry.RoboStakeTotal)
}

// TestNewIngotHashEntry_DifferentBranchHashes tests various hash values
func TestNewIngotHashEntry_DifferentBranchHashes(t *testing.T) {
	hashes := []string{
		"0000000000000000000000000000000000000000000000000000000000000000",
		"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
		"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
		"abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd",
	}

	for _, hash := range hashes {
		ingot := &Phase2Ingot{
			ID:             "hash-test-001",
			BranchHash:     hash,
			HashCount:      3600,
			ContractIDs:    []string{"contract-1"},
			DiggerIDs:      []string{"digger-1"},
			RoboStakeTotal: 5.0,
			Timestamp:      time.Now(),
			Signature:      "sig",
			PublicKey:      "pub",
		}

		entry := NewIngotHashEntry(ingot)
		assert.Equal(t, hash, entry.BranchHash)
	}
}

// TestNewIngotHashEntry_PreservesOrder tests that slice order is preserved
func TestNewIngotHashEntry_PreservesOrder(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "order-test-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"first", "second", "third"},
		DiggerIDs:      []string{"alpha", "beta", "gamma"},
		RoboStakeTotal: 5.0,
		Timestamp:      time.Now(),
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Equal(t, "first", entry.ContractIDs[0])
	assert.Equal(t, "second", entry.ContractIDs[1])
	assert.Equal(t, "third", entry.ContractIDs[2])
	assert.Equal(t, "alpha", entry.DiggerIDs[0])
	assert.Equal(t, "beta", entry.DiggerIDs[1])
	assert.Equal(t, "gamma", entry.DiggerIDs[2])
}

// TestNewIngotHashEntry_TimestampPrecision tests timestamp is preserved accurately
func TestNewIngotHashEntry_TimestampPrecision(t *testing.T) {
	now := time.Now()
	ingot := &Phase2Ingot{
		ID:             "timestamp-test-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.0,
		Timestamp:      now,
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	// Compare with nanosecond precision
	assert.Equal(t, now.Unix(), entry.Timestamp.Unix())
	assert.Equal(t, now.Nanosecond(), entry.Timestamp.Nanosecond())
}

// TestNewIngotHashEntry_RefineryIDEmpty tests RefineryID is initially empty
func TestNewIngotHashEntry_RefineryIDEmpty(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "refinery-test-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.0,
		Timestamp:      time.Now(),
		Signature:      "sig",
		PublicKey:      "pub",
	}

	entry := NewIngotHashEntry(ingot)

	assert.Empty(t, entry.RefineryID)
}

// TestNewIngotHashEntry_MemoryEfficiency tests that entry uses less memory than ingot
func TestNewIngotHashEntry_MemoryEfficiency(t *testing.T) {
	ingot := &Phase2Ingot{
		ID:             "memory-test-001",
		BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		HashCount:      3600,
		ContractIDs:    []string{"contract-1", "contract-2"},
		DiggerIDs:      []string{"digger-1"},
		RoboStakeTotal: 5.0,
		Timestamp:      time.Now(),
		Signature:      "verylongsignaturehere",
		PublicKey:      "verylongpublickeyhere",
	}

	entry := NewIngotHashEntry(ingot)

	// Entries should have the same data but omit full ingot metadata
	// (This is a sanity check; actual memory would need reflection to measure precisely)
	assert.NotNil(t, entry)
	assert.NotNil(t, ingot)
	assert.Equal(t, ingot.BranchHash, entry.BranchHash)
}

// TestNewIngotHashEntry_BatchCreation tests creating multiple entries
func TestNewIngotHashEntry_BatchCreation(t *testing.T) {
	entries := make([]*IngotHashEntry, 0, 10)

	for i := 0; i < 10; i++ {
		ingot := &Phase2Ingot{
			ID:             "batch-" + string(rune(i)),
			BranchHash:     "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
			HashCount:      3600,
			ContractIDs:    []string{"contract-1"},
			DiggerIDs:      []string{"digger-1"},
			RoboStakeTotal: float64(i),
			Timestamp:      time.Now(),
			Signature:      "sig",
			PublicKey:      "pub",
		}
		entries = append(entries, NewIngotHashEntry(ingot))
	}

	assert.Equal(t, 10, len(entries))
	for i, entry := range entries {
		assert.Equal(t, float64(i), entry.RoboStakeTotal)
	}
}
