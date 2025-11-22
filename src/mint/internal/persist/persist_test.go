package persist

import (
	"context"
	"log/slog"
	"os"
	"path/filepath"
	"testing"
	"time"

	"b2b/mint/internal/mint"
	"b2b/mint/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Helper to create test logger
func createTestLogger() *slog.Logger {
	return slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
}

// Helper to create test directory
func createTestDir(t *testing.T) string {
	dir, err := os.MkdirTemp("", "persist-test-*")
	require.NoError(t, err, "failed to create temp directory")
	t.Cleanup(func() { os.RemoveAll(dir) })
	return dir
}

// ============================================================================
// IngotStore Tests
// ============================================================================

func TestNewIngotStore_Success(t *testing.T) {
	dir := createTestDir(t)
	logger := createTestLogger()

	store, err := NewIngotStore(dir, logger)

	assert.NoError(t, err)
	assert.NotNil(t, store)
	assert.Equal(t, dir, store.dir)
}

func TestNewIngotStore_CreatesDirectory(t *testing.T) {
	logger := createTestLogger()
	dir := filepath.Join(os.TempDir(), "persist-test-new-"+time.Now().Format("20060102150405"))

	store, err := NewIngotStore(dir, logger)

	require.NoError(t, err)
	defer os.RemoveAll(dir)

	assert.NotNil(t, store)
	_, err = os.Stat(dir)
	assert.NoError(t, err, "directory should exist")
}

func TestIngotStore_WriteBatch_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	entries := []*models.IngotHashEntry{
		{BranchHash: "aaa111", RoboStakeTotal: 1.5},
		{BranchHash: "bbb222", RoboStakeTotal: 2.5},
	}

	err := store.WriteBatch("batch-001", entries)

	assert.NoError(t, err)

	// Verify file exists
	filePath := filepath.Join(dir, "batch-001.json")
	_, err = os.Stat(filePath)
	assert.NoError(t, err, "batch file should exist")
}

func TestIngotStore_WriteBatch_AtomicWrite(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	entries := []*models.IngotHashEntry{
		{BranchHash: "test", RoboStakeTotal: 1.0},
	}

	err := store.WriteBatch("atomic-test", entries)
	assert.NoError(t, err)

	// Verify no temp files left behind
	files, _ := os.ReadDir(dir)
	for _, f := range files {
		assert.NotContains(t, f.Name(), ".tmp", "temp files should not persist")
	}
}

func TestIngotStore_ReadBatch_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	originalEntries := []*models.IngotHashEntry{
		{BranchHash: "hash1", RoboStakeTotal: 10.5},
		{BranchHash: "hash2", RoboStakeTotal: 20.5},
		{BranchHash: "hash3", RoboStakeTotal: 30.5},
	}

	store.WriteBatch("read-test", originalEntries)
	readEntries, err := store.ReadBatch("read-test")

	assert.NoError(t, err)
	assert.Equal(t, len(originalEntries), len(readEntries))
	assert.Equal(t, originalEntries[0].BranchHash, readEntries[0].BranchHash)
	assert.Equal(t, originalEntries[2].RoboStakeTotal, readEntries[2].RoboStakeTotal)
}

func TestIngotStore_ReadBatch_NotFound(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	_, err := store.ReadBatch("nonexistent")

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "failed to read batch file")
}

func TestIngotStore_DeleteBatch_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	entries := []*models.IngotHashEntry{{BranchHash: "del", RoboStakeTotal: 1.0}}
	store.WriteBatch("delete-me", entries)

	err := store.DeleteBatch("delete-me")
	assert.NoError(t, err)

	// Verify file is gone
	filePath := filepath.Join(dir, "delete-me.json")
	_, err = os.Stat(filePath)
	assert.Error(t, err, "file should not exist after delete")
}

func TestIngotStore_DeleteBatch_NotFound_NoError(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	// Should not error even if batch doesn't exist
	err := store.DeleteBatch("never-existed")
	assert.NoError(t, err, "deleting nonexistent batch should not error")
}

func TestIngotStore_ReadAllBatches_Empty(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	batches, err := store.ReadAllBatches()

	assert.NoError(t, err)
	assert.Equal(t, 0, len(batches))
}

func TestIngotStore_ReadAllBatches_Multiple(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	// Create 3 batches
	for i := 0; i < 3; i++ {
		entries := []*models.IngotHashEntry{
			{BranchHash: "hash" + string(rune(i)), RoboStakeTotal: float64(i+1) * 10},
		}
		store.WriteBatch("batch-"+string(rune(i+48)), entries) // 48 is ASCII for '0'
	}

	batches, err := store.ReadAllBatches()

	assert.NoError(t, err)
	assert.Equal(t, 3, len(batches))
	assert.NotNil(t, batches["batch-0"])
	assert.NotNil(t, batches["batch-1"])
	assert.NotNil(t, batches["batch-2"])
}

func TestIngotStore_ReadAllBatches_SkipsTempFiles(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	// Create a batch
	entries := []*models.IngotHashEntry{{BranchHash: "test", RoboStakeTotal: 1.0}}
	store.WriteBatch("valid", entries)

	// Create a temp file manually
	tempPath := filepath.Join(dir, "orphan.json.tmp")
	os.WriteFile(tempPath, []byte("garbage"), 0644)

	batches, err := store.ReadAllBatches()

	assert.NoError(t, err)
	assert.Equal(t, 1, len(batches), "temp file should be skipped")
}

func TestIngotStore_Count_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	entries := []*models.IngotHashEntry{{BranchHash: "test", RoboStakeTotal: 1.0}}

	store.WriteBatch("b1", entries)
	store.WriteBatch("b2", entries)

	count, err := store.Count()

	assert.NoError(t, err)
	assert.Equal(t, 2, count)
}

func TestIngotStore_ConcurrentWrites(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewIngotStore(dir, createTestLogger())

	// Write batches concurrently
	done := make(chan error, 10)
	for i := 0; i < 10; i++ {
		go func(id int) {
			entries := []*models.IngotHashEntry{
				{BranchHash: "hash" + string(rune(id)), RoboStakeTotal: float64(id)},
			}
			done <- store.WriteBatch("batch-"+string(rune(id+48)), entries)
		}(i)
	}

	// Collect results
	for i := 0; i < 10; i++ {
		err := <-done
		assert.NoError(t, err)
	}

	// Verify all batches exist
	count, _ := store.Count()
	assert.Equal(t, 10, count)
}

// ============================================================================
// ProofStore Tests
// ============================================================================

func TestNewProofStore_Success(t *testing.T) {
	dir := createTestDir(t)
	logger := createTestLogger()

	store, err := NewProofStore(dir, logger)

	assert.NoError(t, err)
	assert.NotNil(t, store)
	assert.Equal(t, dir, store.dir)
}

func TestProofStore_WriteProof_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:         "unit-001",
		MerkleRoot:     "aaaa1111",
		RoboStakeTotal: 5.5,
		TreeHeight:     10,
		MintedAt:       time.Now(),
	}

	merkleResult := &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot}

	err := store.WriteProof(unit.UnitID, unit, merkleResult)

	assert.NoError(t, err)

	// Verify file exists
	filePath := filepath.Join(dir, "unit-001.json")
	_, err = os.Stat(filePath)
	assert.NoError(t, err)
}

func TestProofStore_WriteProof_AtomicWrite(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "atomic",
		MerkleRoot: "bbbb2222",
		MintedAt:   time.Now(),
	}

	store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})

	// Verify no temp files left
	files, _ := os.ReadDir(dir)
	for _, f := range files {
		assert.NotContains(t, f.Name(), ".tmp")
	}
}

func TestProofStore_ReadProof_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	originalUnit := &models.Phase3RoboTorqUnit{
		UnitID:         "read-001",
		MerkleRoot:     "cccc3333",
		RoboStakeTotal: 7.5,
		TreeHeight:     12,
		MintedAt:       time.Now(),
	}

	merkleData := &mint.Level2MerkleResult{MerkleRoot: originalUnit.MerkleRoot}
	store.WriteProof(originalUnit.UnitID, originalUnit, merkleData)

	readUnit, merkleResult, err := store.ReadProof("read-001")

	assert.NoError(t, err)
	assert.Equal(t, originalUnit.UnitID, readUnit.UnitID)
	assert.Equal(t, originalUnit.MerkleRoot, readUnit.MerkleRoot)
	assert.Equal(t, originalUnit.RoboStakeTotal, readUnit.RoboStakeTotal)
	assert.NotNil(t, merkleResult)
	assert.Equal(t, originalUnit.MerkleRoot, merkleResult.MerkleRoot)
}

func TestProofStore_ReadProof_NotFound(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	_, _, err := store.ReadProof("nonexistent-proof")

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "proof not found")
}

func TestProofStore_ListProofs_Empty(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	proofs, err := store.ListProofs()

	assert.NoError(t, err)
	assert.Equal(t, 0, len(proofs))
}

func TestProofStore_ListProofs_Multiple(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	// Create 3 proofs
	for i := 0; i < 3; i++ {
		unit := &models.Phase3RoboTorqUnit{
			UnitID:     "unit-" + string(rune(i+48)),
			MerkleRoot: "root" + string(rune(i+48)),
			MintedAt:   time.Now(),
		}
		store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})
	}

	proofs, err := store.ListProofs()

	assert.NoError(t, err)
	assert.Equal(t, 3, len(proofs))
}

func TestProofStore_FindByIngotHash_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "lookup-001",
		MerkleRoot: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		MintedAt:   time.Now(),
	}

	store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot, HashEntries: []*models.IngotHashEntry{{BranchHash: unit.MerkleRoot}}})

	// FindByIngotHash uses MerkleRoot as index key
	unitID, err := store.FindByIngotHash("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")

	assert.NoError(t, err)
	assert.Equal(t, "lookup-001", unitID)
}

func TestProofStore_FindByIngotHash_NotFound(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	_, err := store.FindByIngotHash("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "ingot hash not found")
}

func TestProofStore_Count_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	unit1 := &models.Phase3RoboTorqUnit{UnitID: "u1", MerkleRoot: "1111111111111111111111111111111111111111111111111111111111111111", MintedAt: time.Now()}
	unit2 := &models.Phase3RoboTorqUnit{UnitID: "u2", MerkleRoot: "2222222222222222222222222222222222222222222222222222222222222222", MintedAt: time.Now()}

	store.WriteProof("u1", unit1, &mint.Level2MerkleResult{MerkleRoot: unit1.MerkleRoot})
	store.WriteProof("u2", unit2, &mint.Level2MerkleResult{MerkleRoot: unit2.MerkleRoot})

	count, err := store.Count()

	assert.NoError(t, err)
	assert.Equal(t, 2, count)
}

func TestProofStore_DeleteOlderThan_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	// Create proof from past (MintedAt in past, but PersistedAtTime will be now)
	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "old-proof",
		MerkleRoot: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
		MintedAt:   time.Now().Add(-48 * time.Hour),
	}

	store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})

	// Sleep to ensure time passes
	time.Sleep(100 * time.Millisecond)

	// Delete proofs persisted before "now" (cutoff = now means delete everything persisted before now)
	deleted, err := store.DeleteOlderThan(time.Now())

	assert.NoError(t, err)
	assert.Equal(t, 1, deleted)

	// Verify proof is gone
	count, _ := store.Count()
	assert.Equal(t, 0, count)
}

func TestProofStore_DeleteOlderThan_KeepsRecent(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	// Create recent proof
	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "recent-proof",
		MerkleRoot: "recent-root",
		MintedAt:   time.Now(),
	}

	store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})

	// Try to delete proofs older than 24 hours
	deleted, err := store.DeleteOlderThan(time.Now().Add(-24 * time.Hour))

	assert.NoError(t, err)
	assert.Equal(t, 0, deleted, "recent proof should be kept")

	// Verify proof still exists
	count, _ := store.Count()
	assert.Equal(t, 1, count)
}

func TestProofStore_ExportCSV_Success(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	now := time.Now()
	unit := &models.Phase3RoboTorqUnit{
		UnitID:         "csv-001",
		MerkleRoot:     "root-csv",
		RoboStakeTotal: 15.25,
		TreeHeight:     11,
		MintedAt:       now,
	}

	store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})

	csvPath := filepath.Join(os.TempDir(), "export-test.csv")
	defer os.Remove(csvPath)

	err := store.ExportCSV(csvPath)

	assert.NoError(t, err)

	// Verify CSV was created
	data, err := os.ReadFile(csvPath)
	assert.NoError(t, err)
	assert.Contains(t, string(data), "unit_id")
	assert.Contains(t, string(data), "merkle_root")
	assert.Contains(t, string(data), "csv-001")
}

func TestProofStore_ConcurrentWrites(t *testing.T) {
	dir := createTestDir(t)
	store, _ := NewProofStore(dir, createTestLogger())

	done := make(chan error, 10)
	for i := 0; i < 10; i++ {
		go func(id int) {
			unit := &models.Phase3RoboTorqUnit{
				UnitID:     "concurrent-" + string(rune(id+48)),
				MerkleRoot: "root-" + string(rune(id+48)),
				MintedAt:   time.Now(),
			}
			done <- store.WriteProof(unit.UnitID, unit, &mint.Level2MerkleResult{MerkleRoot: unit.MerkleRoot})
		}(i)
	}

	for i := 0; i < 10; i++ {
		err := <-done
		assert.NoError(t, err)
	}

	count, _ := store.Count()
	assert.Equal(t, 10, count)
}

// ============================================================================
// PersistenceManager Tests
// ============================================================================

func TestNewPersistenceManager_Success(t *testing.T) {
	dir := createTestDir(t)
	logger := createTestLogger()

	pm, err := NewPersistenceManager(dir, logger)

	assert.NoError(t, err)
	assert.NotNil(t, pm)
	assert.NotNil(t, pm.ingotStore)
	assert.NotNil(t, pm.proofStore)
}

func TestNewPersistenceManager_CreatesSubdirectories(t *testing.T) {
	logger := createTestLogger()
	dir := filepath.Join(os.TempDir(), "persist-manager-"+time.Now().Format("20060102150405"))
	defer os.RemoveAll(dir)

	pm, err := NewPersistenceManager(dir, logger)

	require.NoError(t, err)
	assert.NotNil(t, pm, "PersistenceManager should be created successfully")

	// Verify subdirectories created
	assert.DirExists(t, filepath.Join(dir, "ingots"))
	assert.DirExists(t, filepath.Join(dir, "proofs"))
}

func TestPersistenceManager_SaveIngotBatch(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	entries := []*models.IngotHashEntry{
		{BranchHash: "entry1", RoboStakeTotal: 1.0},
	}

	err := pm.SaveIngotBatch("batch-test", entries)

	assert.NoError(t, err)
	assert.Greater(t, pm.metrics.IngotsWritten, int64(0))
}

func TestPersistenceManager_RecoverInFlightIngots_Empty(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	batches, err := pm.RecoverInFlightIngots(context.Background())

	assert.NoError(t, err)
	assert.Equal(t, 0, len(batches))
}

func TestPersistenceManager_RecoverInFlightIngots_Multiple(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	// Save multiple batches
	for i := 0; i < 3; i++ {
		entries := []*models.IngotHashEntry{
			{BranchHash: "entry" + string(rune(i+48)), RoboStakeTotal: float64(i + 1)},
		}
		pm.SaveIngotBatch("batch-"+string(rune(i+48)), entries)
	}

	batches, err := pm.RecoverInFlightIngots(context.Background())

	assert.NoError(t, err)
	assert.Equal(t, 3, len(batches))
}

func TestPersistenceManager_RemoveIngotBatch(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	entries := []*models.IngotHashEntry{{BranchHash: "test", RoboStakeTotal: 1.0}}
	pm.SaveIngotBatch("to-remove", entries)

	err := pm.RemoveIngotBatch("to-remove")

	assert.NoError(t, err)

	batches, _ := pm.RecoverInFlightIngots(context.Background())
	assert.Equal(t, 0, len(batches))
}

func TestPersistenceManager_SavePhase3Proof(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "pm-proof-001",
		MerkleRoot: "root-pm",
		MintedAt:   time.Now(),
	}

	err := pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	assert.NoError(t, err)
	assert.Greater(t, pm.metrics.ProofsWritten, int64(0))
}

func TestPersistenceManager_LookupPhase3Proof(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	originalUnit := &models.Phase3RoboTorqUnit{
		UnitID:     "lookup-pm",
		MerkleRoot: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		MintedAt:   time.Now(),
	}

	pm.SavePhase3Proof(originalUnit, &mint.Level2MerkleResult{})

	unit, merkle, err := pm.LookupPhase3Proof("lookup-pm")

	assert.NoError(t, err)
	assert.Equal(t, originalUnit.UnitID, unit.UnitID)
	assert.NotNil(t, merkle)
}

func TestPersistenceManager_LookupPhase3ProofByIngotHash(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "ingot-lookup",
		MerkleRoot: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
		MintedAt:   time.Now(),
	}

	pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	unitID, err := pm.LookupPhase3ProofByIngotHash("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")

	assert.NoError(t, err)
	assert.Equal(t, "ingot-lookup", unitID)
}

func TestPersistenceManager_ListPhase3Proofs(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	// two distinct proofs
	unitA := &models.Phase3RoboTorqUnit{UnitID: "list-0", MerkleRoot: "1111111111111111111111111111111111111111111111111111111111111111", MintedAt: time.Now()}
	unitB := &models.Phase3RoboTorqUnit{UnitID: "list-1", MerkleRoot: "2222222222222222222222222222222222222222222222222222222222222222", MintedAt: time.Now()}
	pm.SavePhase3Proof(unitA, &mint.Level2MerkleResult{})
	pm.SavePhase3Proof(unitB, &mint.Level2MerkleResult{})

	proofs, err := pm.ListPhase3Proofs()

	assert.NoError(t, err)
	assert.Equal(t, 2, len(proofs))
}

func TestPersistenceManager_Stats(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	entries := []*models.IngotHashEntry{
		{BranchHash: "3333333333333333333333333333333333333333333333333333333333333333", RoboStakeTotal: 1.0},
	}

	pm.SaveIngotBatch("batch", entries)

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "stat-test",
		MerkleRoot: "4444444444444444444444444444444444444444444444444444444444444444",
		MintedAt:   time.Now(),
	}
	pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	stats := pm.Stats()

	assert.Greater(t, stats.IngotsWritten, int64(0))
	assert.Greater(t, stats.ProofsWritten, int64(0))
}

func TestPersistenceManager_Health_Success(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	err := pm.Health(context.Background())

	assert.NoError(t, err)
}

func TestPersistenceManager_Shutdown(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	err := pm.Shutdown(context.Background())

	assert.NoError(t, err)
}

func TestPersistenceManager_ExportProofCSV(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "export-pm",
		MerkleRoot: "export-root",
		MintedAt:   time.Now(),
	}
	pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	csvPath := filepath.Join(os.TempDir(), "pm-export.csv")
	defer os.Remove(csvPath)

	err := pm.ExportProofCSV(context.Background(), csvPath)

	assert.NoError(t, err)
	_, err = os.Stat(csvPath)
	assert.NoError(t, err)
}

func TestPersistenceManager_CleanupOldProofs(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	// Save a proof (will be persisted at current time)
	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "old-cleanup",
		MerkleRoot: "cleanup-root",
		MintedAt:   time.Now(),
	}
	pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	// Verify proof exists
	count1, _ := pm.proofStore.Count()
	assert.Equal(t, 1, count1, "proof should exist before cleanup")

	// Sleep briefly to ensure time passes
	time.Sleep(100 * time.Millisecond)

	// Cleanup proofs older than 0 seconds (everything)
	deleted, err := pm.CleanupOldProofs(0 * time.Second)

	assert.NoError(t, err)
	assert.Equal(t, 1, deleted, "should have deleted 1 proof")

	// Verify proof is gone
	count2, _ := pm.proofStore.Count()
	assert.Equal(t, 0, count2, "proof should be gone after cleanup")
}

func TestPersistenceManager_FullWorkflow(t *testing.T) {
	dir := createTestDir(t)
	pm, _ := NewPersistenceManager(dir, createTestLogger())

	// 1. Save in-flight ingots
	inFlightEntries := []*models.IngotHashEntry{
		{BranchHash: "ingot1", RoboStakeTotal: 5.0},
		{BranchHash: "ingot2", RoboStakeTotal: 5.0},
	}
	pm.SaveIngotBatch("batch-workflow", inFlightEntries)

	// 2. Save Phase3 proof
	unit := &models.Phase3RoboTorqUnit{
		UnitID:         "workflow-001",
		MerkleRoot:     "workflow-root",
		RoboStakeTotal: 10.0,
		MintedAt:       time.Now(),
	}
	pm.SavePhase3Proof(unit, &mint.Level2MerkleResult{})

	// 3. Remove in-flight batch (after successful publish)
	pm.RemoveIngotBatch("batch-workflow")

	// 4. Verify state
	recoveredBatches, _ := pm.RecoverInFlightIngots(context.Background())
	assert.Equal(t, 0, len(recoveredBatches))

	recoveredUnit, _, _ := pm.LookupPhase3Proof("workflow-001")
	assert.Equal(t, unit.UnitID, recoveredUnit.UnitID)

	stats := pm.Stats()
	assert.Greater(t, stats.IngotsWritten, int64(0))
	assert.Greater(t, stats.ProofsWritten, int64(0))
}

func TestPersistenceManager_CrashRecoveryScenario(t *testing.T) {
	dir := createTestDir(t)
	pm1, _ := NewPersistenceManager(dir, createTestLogger())

	// Simulate crash: save ingots but don't publish
	inFlightEntries := []*models.IngotHashEntry{
		{BranchHash: "crash-ingot", RoboStakeTotal: 3.0},
	}
	pm1.SaveIngotBatch("crash-batch", inFlightEntries)

	// Simulate restart: new manager instance
	pm2, _ := NewPersistenceManager(dir, createTestLogger())

	// Recover state
	batches, _ := pm2.RecoverInFlightIngots(context.Background())

	assert.Equal(t, 1, len(batches))
	assert.Equal(t, 1, len(batches["crash-batch"]))
	assert.Equal(t, "crash-ingot", batches["crash-batch"][0].BranchHash)
}
