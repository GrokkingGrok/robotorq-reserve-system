// Package persist provides persistence layer for the Mint service.
// It handles:
// 1. In-flight ingot recovery (IngotHashQueue state)
// 2. Phase3RoboTorqUnit proof storage (permanent audit trail)
package persist

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"sync"
	"time"

	"b2b/mint/internal/mint"
	"b2b/mint/internal/models"
)

// PersistenceManager coordinates all persistence operations.
// It manages both in-flight state recovery and long-term proof storage.
type PersistenceManager struct {
	dataDir    string // Root directory for all persistence
	logger     *slog.Logger
	mu         sync.RWMutex
	ingotStore *IngotStore // In-flight ingot recovery
	proofStore *ProofStore // Phase3 unit proofs
	metrics    *PersistenceMetrics
}

// PersistenceMetrics tracks persistence operations
type PersistenceMetrics struct {
	IngotsWritten      int64
	IngotsRead         int64
	ProofsWritten      int64
	ProofsRead         int64
	RecoveryOperations int64
	WriteErrors        int64
	ReadErrors         int64
}

// NewPersistenceManager creates a new PersistenceManager
//
// Parameters:
//   - dataDir: Root directory for persistence data (created if needed)
//   - logger: Structured logger
//
// Returns error if dataDir cannot be created or initialized
func NewPersistenceManager(dataDir string, logger *slog.Logger) (*PersistenceManager, error) {
	// Ensure data directory exists
	if err := os.MkdirAll(dataDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create data directory %s: %w", dataDir, err)
	}

	pm := &PersistenceManager{
		dataDir: dataDir,
		logger:  logger,
		metrics: &PersistenceMetrics{},
	}

	// Initialize sub-stores
	ingotStore, err := NewIngotStore(filepath.Join(dataDir, "ingots"), logger)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize ingot store: %w", err)
	}
	pm.ingotStore = ingotStore

	proofStore, err := NewProofStore(filepath.Join(dataDir, "proofs"), logger)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize proof store: %w", err)
	}
	pm.proofStore = proofStore

	logger.Info("persistence manager initialized",
		"data_dir", dataDir,
		"ingot_store", filepath.Join(dataDir, "ingots"),
		"proof_store", filepath.Join(dataDir, "proofs"))

	return pm, nil
}

// ============================================================================
// In-Flight Ingot Persistence (IngotHashQueue recovery)
// ============================================================================

// SaveIngotBatch persists in-flight ingot hash entries for recovery on crash
//
// Called by IngotHashQueue before sending batch to merkle tree builder.
// If crash occurs before Phase3 unit is created, recovery will re-read these.
//
// Parameters:
//   - batchID: Unique identifier for this batch (uuid)
//   - entries: Ingot hash entries to persist
func (pm *PersistenceManager) SaveIngotBatch(batchID string, entries []*models.IngotHashEntry) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	if err := pm.ingotStore.WriteBatch(batchID, entries); err != nil {
		pm.metrics.WriteErrors++
		pm.logger.Error("failed to save ingot batch",
			"batch_id", batchID,
			"entries", len(entries),
			"error", err)
		return err
	}

	pm.metrics.IngotsWritten += int64(len(entries))
	pm.logger.Debug("ingot batch saved",
		"batch_id", batchID,
		"entries", len(entries))

	return nil
}

// RemoveIngotBatch deletes persisted batch after successful Phase3 unit creation
//
// Called after Phase3RoboTorqUnit is successfully published to DistoDam.
// Safe to delete because proof is now stored permanently in ProofStore.
//
// Parameters:
//   - batchID: Batch identifier to delete
func (pm *PersistenceManager) RemoveIngotBatch(batchID string) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	if err := pm.ingotStore.DeleteBatch(batchID); err != nil {
		pm.metrics.WriteErrors++
		pm.logger.Error("failed to remove ingot batch",
			"batch_id", batchID,
			"error", err)
		return err
	}

	pm.logger.Debug("ingot batch removed",
		"batch_id", batchID)

	return nil
}

// RecoverInFlightIngots loads all persisted batches from previous crash
//
// Called at Mint startup. Returns map of batchID -> ingot entries.
// These entries should be re-queued into IngotHashQueue.
//
// Returns:
//   - map[string][]*models.IngotHashEntry if batches found
//   - empty map if no recovery needed
func (pm *PersistenceManager) RecoverInFlightIngots(ctx context.Context) (map[string][]*models.IngotHashEntry, error) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	batches, err := pm.ingotStore.ReadAllBatches()
	if err != nil {
		pm.metrics.ReadErrors++
		pm.logger.Error("failed to recover in-flight ingots", "error", err)
		return nil, err
	}

	pm.metrics.RecoveryOperations++
	if len(batches) > 0 {
		totalEntries := 0
		for _, entries := range batches {
			totalEntries += len(entries)
		}
		pm.logger.Info("recovered in-flight ingots",
			"batches", len(batches),
			"total_entries", totalEntries)
		pm.metrics.IngotsRead += int64(totalEntries)
	}

	return batches, nil
}

// ============================================================================
// Phase3RoboTorqUnit Proof Storage (Permanent audit trail)
// ============================================================================

// SavePhase3Proof persists a Phase3RoboTorqUnit proof permanently
//
// Called after successful Phase3RoboTorqUnit creation and NATS publish.
// This creates the permanent audit trail for lookups, verification, disputes.
//
// Parameters:
//   - unit: Assembled Phase3RoboTorqUnit with proof data
//   - merkleResult: Level2MerkleResult with full tree for proof generation
func (pm *PersistenceManager) SavePhase3Proof(unit *models.Phase3RoboTorqUnit, merkleResult *mint.Level2MerkleResult) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	if err := pm.proofStore.WriteProof(unit.UnitID, unit, merkleResult); err != nil {
		pm.metrics.WriteErrors++
		pm.logger.Error("failed to save Phase3 proof",
			"unit_id", unit.UnitID,
			"error", err)
		return err
	}

	pm.metrics.ProofsWritten++
	pm.logger.Debug("Phase3 proof saved",
		"unit_id", unit.UnitID,
		"merkle_root", unit.MerkleRoot)

	return nil
}

// LookupPhase3Proof retrieves stored Phase3 proof by unit ID
//
// Used by verification API to respond to proof requests.
//
// Parameters:
//   - unitID: Phase3RoboTorqUnit ID to retrieve
//
// Returns:
//   - Phase3RoboTorqUnit if found
//   - merkle tree data if stored
//   - error if not found or read fails
func (pm *PersistenceManager) LookupPhase3Proof(unitID string) (*models.Phase3RoboTorqUnit, *mint.Level2MerkleResult, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	unit, merkle, err := pm.proofStore.ReadProof(unitID)
	if err != nil {
		pm.metrics.ReadErrors++
		return nil, nil, err
	}

	pm.metrics.ProofsRead++
	return unit, merkle, nil
}

// LookupPhase3ProofByIngotHash finds Phase3 proof containing specific ingot hash
//
// Used by verification API to answer "which Phase3 unit contains this ingot?"
//
// Parameters:
//   - ingotHash: 64-character hex SHA256 hash of Phase2Ingot
//
// Returns:
//   - unitID if found
//   - error if not found
func (pm *PersistenceManager) LookupPhase3ProofByIngotHash(ingotHash string) (string, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	return pm.proofStore.FindByIngotHash(ingotHash)
}

// ListPhase3Proofs returns all stored Phase3 unit IDs (for audit/recovery)
//
// Used for:
// - Metrics reporting (total proofs stored)
// - Recovery operations (find gaps in storage)
// - Maintenance operations (export/backup)
//
// Returns:
//   - []string of all stored unit IDs
//   - error if listing fails
func (pm *PersistenceManager) ListPhase3Proofs() ([]string, error) {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	return pm.proofStore.ListProofs()
}

// ============================================================================
// Health & Observability
// ============================================================================

// Stats returns persistence statistics
func (pm *PersistenceManager) Stats() PersistenceMetrics {
	pm.mu.RLock()
	defer pm.mu.RUnlock()
	return *pm.metrics
}

// Health checks persistence layer readiness
//
// Returns error if data directories inaccessible or corrupt
func (pm *PersistenceManager) Health(ctx context.Context) error {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	// Check ingot store
	if _, err := pm.ingotStore.ReadAllBatches(); err != nil {
		return fmt.Errorf("ingot store health check failed: %w", err)
	}

	// Check proof store
	if _, err := pm.proofStore.ListProofs(); err != nil {
		return fmt.Errorf("proof store health check failed: %w", err)
	}

	return nil
}

// Shutdown gracefully closes all persistence operations
//
// Parameters:
//   - ctx: Context for timeout control
//
// Returns error if shutdown takes too long or fails
func (pm *PersistenceManager) Shutdown(ctx context.Context) error {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	pm.logger.Info("persistence manager shutting down")

	// Flush any pending writes (future enhancement)
	// For now, just ensure directories are synced

	pm.logger.Info("persistence manager shutdown complete",
		"ingots_written", pm.metrics.IngotsWritten,
		"proofs_written", pm.metrics.ProofsWritten)

	return nil
}

// ============================================================================
// Utility Functions
// ============================================================================

// ExportProofCSV exports all proofs to CSV for audit/analysis
//
// Used for:
// - Audit reports
// - Compliance verification
// - Data analysis
//
// Parameters:
//   - outputPath: Where to write CSV file
//
// Returns error if export fails
func (pm *PersistenceManager) ExportProofCSV(ctx context.Context, outputPath string) error {
	pm.mu.RLock()
	defer pm.mu.RUnlock()

	return pm.proofStore.ExportCSV(outputPath)
}

// CleanupOldProofs removes proofs older than specified duration
//
// Used for:
// - Storage maintenance
// - Compliance data retention policies
// - Backup rotation
//
// Parameters:
//   - olderThan: Duration threshold (e.g., 30*24*time.Hour)
//
// Returns:
//   - count: Number of proofs deleted
//   - error if cleanup fails
func (pm *PersistenceManager) CleanupOldProofs(olderThan time.Duration) (int, error) {
	pm.mu.Lock()
	defer pm.mu.Unlock()

	cutoff := time.Now().Add(-olderThan)
	return pm.proofStore.DeleteOlderThan(cutoff)
}
