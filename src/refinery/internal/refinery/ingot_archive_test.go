// ingot_archive_test.go - Unit tests for IngotArchive
package refinery

import (
	"fmt"
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/refinery/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestNewIngotArchive(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	assert.NotNil(t, archive)
	assert.NotNil(t, archive.jtuHashes)
	assert.NotNil(t, archive.fullIngots)
	assert.Equal(t, 0, len(archive.jtuHashes))
	assert.Equal(t, 0, len(archive.fullIngots))
}

func TestIngotArchive_Store(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Create test ingot with 3600 units
	ingot := createTestIngot("test-ingot-001", 3600)

	err := archive.Store(ingot)
	require.NoError(t, err)

	// Verify hashes stored
	stats := archive.Stats()
	assert.Equal(t, 1, stats.TotalIngotsWithHashes)
	assert.Equal(t, 3600, stats.TotalJTUHashes)
	assert.Equal(t, 1, stats.FullIngotsAvailable)
}

func TestIngotArchive_StoreNilIngot(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	err := archive.Store(nil)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "cannot store nil ingot")
}

func TestIngotArchive_StoreNilUnit(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Create ingot manually with nil unit
	units := make([]*models.JouleTorqUnit, 10)
	for i := 0; i < 10; i++ {
		if i == 5 {
			units[i] = nil // Inject nil unit at index 5
		} else {
			units[i] = models.NewJouleTorqUnit(
				"test-contract",
				1,
				i,
				15.0,
				0.01,
				"test-digger",
				"stub-signature",
				"stub-pubkey",
			)
		}
	}

	ingot := &models.TokenTorqIngot{
		IngotID: "test-ingot-002",
		Units:   units,
	}

	err := archive.Store(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "nil unit at index 5")
}

func TestIngotArchive_GetJTUHash(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	ingot := createTestIngot("test-ingot-003", 100)
	require.NoError(t, archive.Store(ingot))

	// Test valid hash retrieval
	hash, found := archive.GetJTUHash("test-ingot-003", 42)
	assert.True(t, found)
	assert.Equal(t, ingot.Units[42].Hash, hash)

	// Test first and last
	hash0, found0 := archive.GetJTUHash("test-ingot-003", 0)
	assert.True(t, found0)
	assert.Equal(t, ingot.Units[0].Hash, hash0)

	hash99, found99 := archive.GetJTUHash("test-ingot-003", 99)
	assert.True(t, found99)
	assert.Equal(t, ingot.Units[99].Hash, hash99)
}

func TestIngotArchive_GetJTUHash_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Non-existent ingot
	hash, found := archive.GetJTUHash("non-existent", 0)
	assert.False(t, found)
	assert.Empty(t, hash)

	// Existing ingot, invalid index
	ingot := createTestIngot("test-ingot-004", 10)
	require.NoError(t, archive.Store(ingot))

	hash, found = archive.GetJTUHash("test-ingot-004", 99)
	assert.False(t, found)
	assert.Empty(t, hash)
}

func TestIngotArchive_GetJTU(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	ingot := createTestIngot("test-ingot-005", 100)
	require.NoError(t, archive.Store(ingot))

	// Test valid JTU retrieval
	unit, found := archive.GetJTU("test-ingot-005", 42)
	assert.True(t, found)
	assert.NotNil(t, unit)
	assert.Equal(t, ingot.Units[42].TokenID, unit.TokenID)
	assert.Equal(t, ingot.Units[42].Hash, unit.Hash)
}

func TestIngotArchive_GetJTU_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Non-existent ingot
	unit, found := archive.GetJTU("non-existent", 0)
	assert.False(t, found)
	assert.Nil(t, unit)

	// Existing ingot, out of bounds index
	ingot := createTestIngot("test-ingot-006", 10)
	require.NoError(t, archive.Store(ingot))

	unit, found = archive.GetJTU("test-ingot-006", 99)
	assert.False(t, found)
	assert.Nil(t, unit)

	unit, found = archive.GetJTU("test-ingot-006", -1)
	assert.False(t, found)
	assert.Nil(t, unit)
}

func TestIngotArchive_GetIngot(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	original := createTestIngot("test-ingot-007", 50)
	require.NoError(t, archive.Store(original))

	// Test valid ingot retrieval
	retrieved, found := archive.GetIngot("test-ingot-007")
	assert.True(t, found)
	assert.NotNil(t, retrieved)
	assert.Equal(t, original.IngotID, retrieved.IngotID)
	assert.Equal(t, len(original.Units), len(retrieved.Units))
	assert.Equal(t, original.JouleTorqTotal, retrieved.JouleTorqTotal)
}

func TestIngotArchive_GetIngot_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	ingot, found := archive.GetIngot("non-existent")
	assert.False(t, found)
	assert.Nil(t, ingot)
}

func TestIngotArchive_Expiration(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	ingot := createTestIngot("test-ingot-008", 10)
	require.NoError(t, archive.Store(ingot))

	// Manually expire the ingot
	archive.mu.Lock()
	archive.fullIngots["test-ingot-008"].ExpiresAt = time.Now().Add(-1 * time.Hour)
	archive.mu.Unlock()

	// Full ingot should not be available
	retrievedIngot, found := archive.GetIngot("test-ingot-008")
	assert.False(t, found)
	assert.Nil(t, retrievedIngot)

	// Full JTU should not be available
	unit, found := archive.GetJTU("test-ingot-008", 5)
	assert.False(t, found)
	assert.Nil(t, unit)

	// But hash should still be available (permanent)
	hash, found := archive.GetJTUHash("test-ingot-008", 5)
	assert.True(t, found)
	assert.NotEmpty(t, hash)
}

func TestIngotArchive_CleanupExpired(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Store 3 ingots
	for i := 0; i < 3; i++ {
		ingot := createTestIngot(string(rune('a'+i))+"-ingot", 10)
		require.NoError(t, archive.Store(ingot))
	}

	stats := archive.Stats()
	assert.Equal(t, 3, stats.FullIngotsAvailable)

	// Expire 2 of them
	archive.mu.Lock()
	archive.fullIngots["a-ingot"].ExpiresAt = time.Now().Add(-1 * time.Hour)
	archive.fullIngots["b-ingot"].ExpiresAt = time.Now().Add(-1 * time.Hour)
	archive.mu.Unlock()

	// Run cleanup
	cleaned := archive.CleanupExpired()
	assert.Equal(t, 2, cleaned)

	// Verify stats
	stats = archive.Stats()
	assert.Equal(t, 1, stats.FullIngotsAvailable)   // Only 1 full ingot remains
	assert.Equal(t, 3, stats.TotalIngotsWithHashes) // All 3 hashes remain
	assert.Equal(t, 30, stats.TotalJTUHashes)       // 3 ingots × 10 units = 30 hashes
}

func TestIngotArchive_Stats(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Empty archive
	stats := archive.Stats()
	assert.Equal(t, 0, stats.TotalIngotsWithHashes)
	assert.Equal(t, 0, stats.TotalJTUHashes)
	assert.Equal(t, 0, stats.FullIngotsAvailable)

	// Add ingots with different unit counts
	require.NoError(t, archive.Store(createTestIngot("ingot-1", 100)))
	require.NoError(t, archive.Store(createTestIngot("ingot-2", 200)))
	require.NoError(t, archive.Store(createTestIngot("ingot-3", 300)))

	stats = archive.Stats()
	assert.Equal(t, 3, stats.TotalIngotsWithHashes)
	assert.Equal(t, 600, stats.TotalJTUHashes) // 100 + 200 + 300
	assert.Equal(t, 3, stats.FullIngotsAvailable)
}

func TestIngotArchive_ConcurrentAccess(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	ingot := createTestIngot("concurrent-ingot", 100)
	require.NoError(t, archive.Store(ingot))

	// Concurrent reads (should not panic)
	done := make(chan bool, 10)
	for i := 0; i < 10; i++ {
		go func(idx int) {
			hash, found := archive.GetJTUHash("concurrent-ingot", idx)
			assert.True(t, found)
			assert.NotEmpty(t, hash)

			unit, found := archive.GetJTU("concurrent-ingot", idx)
			assert.True(t, found)
			assert.NotNil(t, unit)

			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}
}

func TestIngotArchive_StartCleanupRoutine(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)

	// Start cleanup with very short interval
	stop := archive.StartCleanupRoutine(100 * time.Millisecond)
	defer close(stop)

	// Add expired ingot
	ingot := createTestIngot("cleanup-test", 10)
	require.NoError(t, archive.Store(ingot))

	archive.mu.Lock()
	archive.fullIngots["cleanup-test"].ExpiresAt = time.Now().Add(-1 * time.Hour)
	archive.mu.Unlock()

	// Wait for cleanup to run
	time.Sleep(200 * time.Millisecond)

	// Verify ingot was cleaned up
	stats := archive.Stats()
	assert.Equal(t, 0, stats.FullIngotsAvailable)
	assert.Equal(t, 1, stats.TotalIngotsWithHashes) // Hashes remain
}

// Helper function to create test ingot
func createTestIngot(ingotID string, numUnits int) *models.TokenTorqIngot {
	units := make([]*models.JouleTorqUnit, numUnits)
	var totalJoules float64
	var totalRobo float64

	for i := 0; i < numUnits; i++ {
		unit := models.NewJouleTorqUnit(
			"test-contract",
			1,
			i,
			15.0, // joules
			0.01, // robo stake
			"test-digger",
			"stub-signature",
			"stub-pubkey",
		)
		units[i] = unit
		totalJoules += 15.0
		totalRobo += 0.01
	}

	// If less than 3600 units, create ingot struct manually (for testing)
	// NewTokenTorqIngot requires exactly 3600 units
	if numUnits != 3600 {
		ingot := &models.TokenTorqIngot{
			IngotID:        ingotID,
			Units:          units,
			JouleTorqTotal: totalJoules,
			RoboStakeTotal: totalRobo,
			ContractIDs:    []string{"test-contract"},
			MintedAt:       time.Now().UTC(),
		}
		return ingot
	}

	// For 3600 units, use proper constructor
	ingot, err := models.NewTokenTorqIngot(units)
	if err != nil {
		panic(fmt.Sprintf("createTestIngot failed: %v", err))
	}
	ingot.IngotID = ingotID

	return ingot
}
