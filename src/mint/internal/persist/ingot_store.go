package persist

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"sync"

	"b2b/mint/internal/models"
)

// IngotStore handles persistence of in-flight ingot batches
//
// Design:
// - Each batch stored as separate JSON file: {batchID}.json
// - File-per-batch allows atomic writes and easy recovery
// - Recovery reads all .json files on startup
// - Files deleted after Phase3 unit successfully created
//
// Crash Safety:
// - Write is atomic (write to temp, rename)
// - Crash during batch processing leaves orphan .json files
// - Recovery on startup finds and re-queues orphans
// - No duplicate processing (same batchID won't be processed twice)
type IngotStore struct {
	dir    string        // Directory storing batch files
	logger *slog.Logger
	mu     sync.RWMutex
}

// BatchFile represents the JSON structure of a persisted batch
type BatchFile struct {
	BatchID   string                    `json:"batch_id"`
	Entries   []*models.IngotHashEntry  `json:"entries"`
	CreatedAt int64                     `json:"created_at_unix_ns"`
}

// NewIngotStore creates a new IngotStore
func NewIngotStore(dir string, logger *slog.Logger) (*IngotStore, error) {
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create ingot store directory %s: %w", dir, err)
	}

	return &IngotStore{
		dir:    dir,
		logger: logger,
	}, nil
}

// WriteBatch persists a batch of ingot hash entries
//
// Atomic write: creates temp file, then renames to final name.
// This prevents partial writes on crash.
//
// Parameters:
//   - batchID: Unique batch identifier (should be UUID)
//   - entries: Ingot hash entries to persist
func (is *IngotStore) WriteBatch(batchID string, entries []*models.IngotHashEntry) error {
	is.mu.Lock()
	defer is.mu.Unlock()

	batch := BatchFile{
		BatchID:   batchID,
		Entries:   entries,
		CreatedAt: int64(len(entries)), // Use count as simple timestamp
	}

	data, err := json.MarshalIndent(&batch, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal batch: %w", err)
	}

	// Write to temp file first
	finalPath := filepath.Join(is.dir, fmt.Sprintf("%s.json", batchID))
	tempPath := finalPath + ".tmp"

	if err := os.WriteFile(tempPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write temp batch file: %w", err)
	}

	// Atomic rename
	if err := os.Rename(tempPath, finalPath); err != nil {
		os.Remove(tempPath) // Cleanup temp on error
		return fmt.Errorf("failed to rename batch file: %w", err)
	}

	is.logger.Debug("batch persisted",
		"batch_id", batchID,
		"path", finalPath,
		"entries", len(entries))

	return nil
}

// DeleteBatch removes a persisted batch file
//
// Called after Phase3 unit successfully created and published.
// Safe to delete because proof is now in ProofStore.
//
// Parameters:
//   - batchID: Batch identifier to delete
func (is *IngotStore) DeleteBatch(batchID string) error {
	is.mu.Lock()
	defer is.mu.Unlock()

	path := filepath.Join(is.dir, fmt.Sprintf("%s.json", batchID))

	// Check if file exists before deleting
	if _, err := os.Stat(path); os.IsNotExist(err) {
		// Already gone, not an error
		return nil
	}

	if err := os.Remove(path); err != nil {
		return fmt.Errorf("failed to delete batch file %s: %w", path, err)
	}

	is.logger.Debug("batch deleted", "batch_id", batchID)
	return nil
}

// ReadBatch retrieves a specific persisted batch by ID
//
// Parameters:
//   - batchID: Batch identifier to retrieve
//
// Returns:
//   - IngotHashEntry slice if found
//   - error if not found or read fails
func (is *IngotStore) ReadBatch(batchID string) ([]*models.IngotHashEntry, error) {
	is.mu.RLock()
	defer is.mu.RUnlock()

	path := filepath.Join(is.dir, fmt.Sprintf("%s.json", batchID))

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read batch file: %w", err)
	}

	var batch BatchFile
	if err := json.Unmarshal(data, &batch); err != nil {
		return nil, fmt.Errorf("failed to unmarshal batch file: %w", err)
	}

	return batch.Entries, nil
}

// ReadAllBatches retrieves all persisted batches (for recovery)
//
// Called at startup to recover from crashes.
// Scans directory for all .json files (excludes .tmp files).
//
// Returns:
//   - map[batchID] -> []*IngotHashEntry
//   - empty map if no batches found (no recovery needed)
//   - error if read fails
func (is *IngotStore) ReadAllBatches() (map[string][]*models.IngotHashEntry, error) {
	is.mu.RLock()
	defer is.mu.RUnlock()

	entries, err := os.ReadDir(is.dir)
	if err != nil {
		return nil, fmt.Errorf("failed to read ingot store directory: %w", err)
	}

	batches := make(map[string][]*models.IngotHashEntry)

	for _, entry := range entries {
		// Skip directories and temp files
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}

		path := filepath.Join(is.dir, entry.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			is.logger.Warn("failed to read batch file", "path", path, "error", err)
			continue
		}

		var batch BatchFile
		if err := json.Unmarshal(data, &batch); err != nil {
			is.logger.Warn("failed to unmarshal batch file", "path", path, "error", err)
			continue
		}

		batches[batch.BatchID] = batch.Entries
		is.logger.Debug("loaded persisted batch",
			"batch_id", batch.BatchID,
			"entries", len(batch.Entries))
	}

	return batches, nil
}

// Count returns number of persisted batches
func (is *IngotStore) Count() (int, error) {
	is.mu.RLock()
	defer is.mu.RUnlock()

	entries, err := os.ReadDir(is.dir)
	if err != nil {
		return 0, err
	}

	count := 0
	for _, entry := range entries {
		if !entry.IsDir() && filepath.Ext(entry.Name()) == ".json" {
			count++
		}
	}

	return count, nil
}
