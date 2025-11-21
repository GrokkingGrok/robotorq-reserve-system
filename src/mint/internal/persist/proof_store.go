package persist

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"sync"
	"time"

	"b2b/mint/internal/models"
)

// ProofStore handles persistence of Phase3RoboTorqUnit proofs
//
// Design:
// - Each proof stored as separate JSON file: {unitID}.json
// - Contains Phase3RoboTorqUnit + merkle tree data (Level2MerkleResult)
// - File-per-unit allows atomic writes and efficient lookups
// - Reverse index: ingotHash -> unitID (in-memory, rebuilt on startup)
//
// Use Cases:
// - Verification API: GET /verify/proof/{unitID}
// - Audit trail: Historical record of all minted RoboTorq units
// - Dispute resolution: Prove which unit contains specific ingot
// - CSV export: Compliance reporting
//
// Storage & Retrieval:
// - Write: Atomic (temp + rename) after successful publishing
// - Read: Fast file lookup by unitID
// - Index: Reverse lookup by ingot hash (memory-based for speed)
// - Cleanup: Delete proofs older than retention period
type ProofStore struct {
	dir         string // Directory storing proof files
	logger      *slog.Logger
	mu          sync.RWMutex
	ingotIndex  map[string]string // ingotHash -> unitID (reverse lookup)
	rebuildOnce sync.Once
}

// ProofFile represents the JSON structure of a persisted proof
type ProofFile struct {
	Unit            *models.Phase3RoboTorqUnit `json:"unit"`
	MerkleTreeJSON  json.RawMessage            `json:"merkle_tree"` // Store as raw JSON
	PersistedAt     int64                      `json:"persisted_at_unix"`
	PersistedAtTime time.Time                  `json:"persisted_at_time"`
}

// NewProofStore creates a new ProofStore
func NewProofStore(dir string, logger *slog.Logger) (*ProofStore, error) {
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create proof store directory %s: %w", dir, err)
	}

	ps := &ProofStore{
		dir:        dir,
		logger:     logger,
		ingotIndex: make(map[string]string),
	}

	// Index is rebuilt lazily on first lookup
	return ps, nil
}

// WriteProof persists a Phase3RoboTorqUnit proof permanently
//
// Atomic write: creates temp file, then renames to final name.
// This prevents partial writes on crash.
//
// Parameters:
//   - unitID: Phase3RoboTorqUnit ID (cache key)
//   - unit: Complete Phase3RoboTorqUnit with all fields
//   - merkleResult: Level2MerkleResult with tree for proof generation (stored as JSON)
func (ps *ProofStore) WriteProof(unitID string, unit *models.Phase3RoboTorqUnit, merkleResult interface{}) error {
	ps.mu.Lock()
	defer ps.mu.Unlock()

	// Serialize merkle tree as JSON
	merkleJSON, err := json.Marshal(merkleResult)
	if err != nil {
		return fmt.Errorf("failed to marshal merkle result: %w", err)
	}

	proof := ProofFile{
		Unit:            unit,
		MerkleTreeJSON:  merkleJSON,
		PersistedAt:     time.Now().UnixNano(),
		PersistedAtTime: time.Now(),
	}

	data, err := json.MarshalIndent(&proof, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal proof: %w", err)
	}

	// Write to temp file first
	finalPath := filepath.Join(ps.dir, fmt.Sprintf("%s.json", unitID))
	tempPath := finalPath + ".tmp"

	if err := os.WriteFile(tempPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write temp proof file: %w", err)
	}

	// Atomic rename
	if err := os.Rename(tempPath, finalPath); err != nil {
		os.Remove(tempPath) // Cleanup temp on error
		return fmt.Errorf("failed to rename proof file: %w", err)
	}

	// Update in-memory reverse index
	ps.ingotIndex[unit.MerkleRoot] = unitID

	ps.logger.Debug("proof persisted",
		"unit_id", unitID,
		"path", finalPath,
		"merkle_root", unit.MerkleRoot)

	return nil
}

// ReadProof retrieves a specific proof by unit ID
//
// Parameters:
//   - unitID: Phase3RoboTorqUnit ID to retrieve
//
// Returns:
//   - Phase3RoboTorqUnit if found
//   - Level2MerkleResult (as raw JSON) if stored
//   - error if not found or read fails
func (ps *ProofStore) ReadProof(unitID string) (*models.Phase3RoboTorqUnit, interface{}, error) {
	ps.mu.RLock()
	defer ps.mu.RUnlock()

	path := filepath.Join(ps.dir, fmt.Sprintf("%s.json", unitID))

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, fmt.Errorf("proof not found: %w", err)
	}

	var proof ProofFile
	if err := json.Unmarshal(data, &proof); err != nil {
		return nil, nil, fmt.Errorf("failed to unmarshal proof file: %w", err)
	}

	return proof.Unit, proof.MerkleTreeJSON, nil
}

// FindByIngotHash finds unit ID containing specific ingot hash
//
// Uses reverse index (ingotHash -> unitID).
// Index is built lazily on first lookup, then maintained incrementally.
//
// Parameters:
//   - ingotHash: 64-character hex SHA256 hash of Phase2Ingot
//
// Returns:
//   - unitID if found
//   - error if not found
func (ps *ProofStore) FindByIngotHash(ingotHash string) (string, error) {
	ps.mu.Lock()
	defer ps.mu.Unlock()

	// First-time: rebuild index from all proof files
	ps.rebuildOnce.Do(func() {
		ps.rebuildIngotIndex()
	})

	unitID, found := ps.ingotIndex[ingotHash]
	if !found {
		return "", fmt.Errorf("ingot hash not found in any proof: %s", ingotHash)
	}

	return unitID, nil
}

// rebuildIngotIndex scans all proof files and builds reverse index
// This is called once on startup for consistency.
//
// Ingot hash index maps: ingotHash -> unitID
// Allows fast reverse lookup without scanning all proof files.
func (ps *ProofStore) rebuildIngotIndex() {
	entries, err := os.ReadDir(ps.dir)
	if err != nil {
		ps.logger.Warn("failed to read proof store directory", "error", err)
		return
	}

	count := 0
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}

		path := filepath.Join(ps.dir, entry.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			ps.logger.Warn("failed to read proof file during index rebuild", "path", path, "error", err)
			continue
		}

		var proof ProofFile
		if err := json.Unmarshal(data, &proof); err != nil {
			ps.logger.Warn("failed to unmarshal proof file during index rebuild", "path", path, "error", err)
			continue
		}

		// Add to index: merkle_root -> unitID
		if proof.Unit != nil {
			ps.ingotIndex[proof.Unit.MerkleRoot] = proof.Unit.UnitID
			count++
		}
	}

	ps.logger.Info("proof store index rebuilt", "proofs_indexed", count)
}

// ListProofs returns all stored unit IDs
//
// Used for:
// - Metrics (total proofs stored)
// - Recovery operations (find gaps)
// - Audit/export operations
//
// Returns:
//   - []string of unit IDs
//   - error if listing fails
func (ps *ProofStore) ListProofs() ([]string, error) {
	ps.mu.RLock()
	defer ps.mu.RUnlock()

	entries, err := os.ReadDir(ps.dir)
	if err != nil {
		return nil, fmt.Errorf("failed to read proof store directory: %w", err)
	}

	var unitIDs []string
	for _, entry := range entries {
		if !entry.IsDir() && filepath.Ext(entry.Name()) == ".json" {
			// Filename is {unitID}.json
			unitID := entry.Name()[:len(entry.Name())-5] // Remove .json
			unitIDs = append(unitIDs, unitID)
		}
	}

	return unitIDs, nil
}

// DeleteOlderThan removes proofs with PersistedAt before cutoff time
//
// Used for retention policy enforcement (e.g., delete proofs > 30 days old).
//
// Parameters:
//   - cutoff: Time threshold (proofs before this are deleted)
//
// Returns:
//   - count: Number of proofs deleted
//   - error if cleanup fails
func (ps *ProofStore) DeleteOlderThan(cutoff time.Time) (int, error) {
	ps.mu.Lock()
	defer ps.mu.Unlock()

	entries, err := os.ReadDir(ps.dir)
	if err != nil {
		return 0, fmt.Errorf("failed to read proof store directory: %w", err)
	}

	deleted := 0
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}

		path := filepath.Join(ps.dir, entry.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			ps.logger.Warn("failed to read proof file during cleanup", "path", path, "error", err)
			continue
		}

		var proof ProofFile
		if err := json.Unmarshal(data, &proof); err != nil {
			ps.logger.Warn("failed to unmarshal proof file during cleanup", "path", path, "error", err)
			continue
		}

		// Delete if persisted before cutoff
		if proof.PersistedAtTime.Before(cutoff) {
			if err := os.Remove(path); err != nil {
				ps.logger.Warn("failed to delete old proof file", "path", path, "error", err)
				continue
			}
			deleted++
		}
	}

	ps.logger.Info("proofs cleaned up", "deleted", deleted, "cutoff_time", cutoff)
	return deleted, nil
}

// ExportCSV exports all proofs to CSV for audit/analysis
//
// CSV format:
//
//	unit_id, merkle_root, robo_stake_total, tree_height, minted_at, persisted_at
//
// Parameters:
//   - outputPath: Where to write CSV file
//
// Returns error if export fails
func (ps *ProofStore) ExportCSV(outputPath string) error {
	ps.mu.RLock()
	defer ps.mu.RUnlock()

	entries, err := os.ReadDir(ps.dir)
	if err != nil {
		return fmt.Errorf("failed to read proof store directory: %w", err)
	}

	// Create CSV file
	f, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("failed to create CSV file: %w", err)
	}
	defer f.Close()

	w := csv.NewWriter(f)
	defer w.Flush()

	// Write header
	w.Write([]string{"unit_id", "merkle_root", "robo_stake_total", "tree_height", "minted_at", "persisted_at"})

	// Write rows
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".json" {
			continue
		}

		path := filepath.Join(ps.dir, entry.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			ps.logger.Warn("failed to read proof file during CSV export", "path", path, "error", err)
			continue
		}

		var proof ProofFile
		if err := json.Unmarshal(data, &proof); err != nil {
			ps.logger.Warn("failed to unmarshal proof file during CSV export", "path", path, "error", err)
			continue
		}

		if proof.Unit != nil {
			w.Write([]string{
				proof.Unit.UnitID,
				proof.Unit.MerkleRoot,
				fmt.Sprintf("%.6f", proof.Unit.RoboStakeTotal),
				fmt.Sprintf("%v", proof.Unit.TreeHeight),
				proof.Unit.MintedAt.Format(time.RFC3339Nano),
				proof.PersistedAtTime.Format(time.RFC3339Nano),
			})
		}
	}

	ps.logger.Info("proofs exported to CSV", "path", outputPath)
	return nil
}

// Count returns number of stored proofs
func (ps *ProofStore) Count() (int, error) {
	ps.mu.RLock()
	defer ps.mu.RUnlock()

	proofs, err := ps.ListProofs()
	if err != nil {
		return 0, err
	}

	return len(proofs), nil
}
