// ingot_archive.go - Stores ingot data for JTU lookup
package refinery

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"b2b/refinery/internal/models"
)

// IngotArchive stores ingot data for historical lookups
//
// Storage Strategy:
//   - JTU Hashes: Stored forever (permanent proof chain)
//   - Full Ingot Data: Stored for 30 days (recent detailed lookups)
//   - After 30 days: Ingot data deleted, only hashes remain
//
// Purpose:
//   - Enables DistoDam to query individual JTU hashes when distributing
//   - Supports merkle proof verification for any historical JTU
//   - Balances storage cost (hashes only) vs query capability (full data)
type IngotArchive struct {
	mu sync.RWMutex

	// Permanent hash storage (ingot_id -> jtu_index -> hash)
	// Memory: ~64 bytes per hash × 3600 JTUs × N ingots
	jtuHashes map[string]map[int]string

	// Temporary full ingot storage (ingot_id -> ingot data)
	// Deleted after 30 days to save memory
	fullIngots map[string]*IngotArchiveEntry

	logger *slog.Logger
}

// IngotArchiveEntry wraps an ingot with metadata for expiration
type IngotArchiveEntry struct {
	Ingot     *models.TokenTorqIngot `json:"ingot"`
	CreatedAt time.Time              `json:"created_at"`
	ExpiresAt time.Time              `json:"expires_at"` // 30 days from creation
}

// NewIngotArchive creates a new ingot archive
func NewIngotArchive(logger *slog.Logger) *IngotArchive {
	return &IngotArchive{
		jtuHashes:  make(map[string]map[int]string),
		fullIngots: make(map[string]*IngotArchiveEntry),
		logger:     logger,
	}
}

// Store saves an ingot to the archive
//
// Stores:
//   - All JTU hashes permanently
//   - Full ingot data for 30 days
//
// Parameters:
//   - ingot: The TokenTorqIngot to archive
func (ia *IngotArchive) Store(ingot *models.TokenTorqIngot) error {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	if ingot == nil {
		return fmt.Errorf("cannot store nil ingot")
	}

	ingotID := ingot.IngotID
	now := time.Now()

	// Extract and store all JTU hashes (permanent)
	hashMap := make(map[int]string, len(ingot.Units))
	for i, unit := range ingot.Units {
		if unit == nil {
			return fmt.Errorf("nil unit at index %d in ingot %s", i, ingotID)
		}
		hashMap[i] = unit.Hash
	}
	ia.jtuHashes[ingotID] = hashMap

	// Store full ingot (expires in 30 days)
	ia.fullIngots[ingotID] = &IngotArchiveEntry{
		Ingot:     ingot,
		CreatedAt: now,
		ExpiresAt: now.Add(30 * 24 * time.Hour),
	}

	ia.logger.Info("ingot archived",
		"ingot_id", ingotID,
		"jtu_count", len(ingot.Units),
		"expires_at", ia.fullIngots[ingotID].ExpiresAt.Format(time.RFC3339))

	return nil
}

// GetJTUHash retrieves the hash for a specific JTU (available forever)
//
// Parameters:
//   - ingotID: The ingot identifier
//   - jtuIndex: The JTU index within the ingot (0-3599)
//
// Returns:
//   - hash: The JTU hash (64-char hex)
//   - found: true if hash exists
func (ia *IngotArchive) GetJTUHash(ingotID string, jtuIndex int) (string, bool) {
	ia.mu.RLock()
	defer ia.mu.RUnlock()

	if hashMap, exists := ia.jtuHashes[ingotID]; exists {
		hash, found := hashMap[jtuIndex]
		return hash, found
	}

	return "", false
}

// GetJTU retrieves full JTU details (only if < 30 days old)
//
// Parameters:
//   - ingotID: The ingot identifier
//   - jtuIndex: The JTU index within the ingot (0-3599)
//
// Returns:
//   - unit: The full JouleTorqUnit
//   - found: true if full data available
func (ia *IngotArchive) GetJTU(ingotID string, jtuIndex int) (*models.JouleTorqUnit, bool) {
	ia.mu.RLock()
	defer ia.mu.RUnlock()

	entry, exists := ia.fullIngots[ingotID]
	if !exists || entry == nil {
		return nil, false
	}

	// Check if expired
	if time.Now().After(entry.ExpiresAt) {
		return nil, false
	}

	// Validate index
	if jtuIndex < 0 || jtuIndex >= len(entry.Ingot.Units) {
		return nil, false
	}

	return entry.Ingot.Units[jtuIndex], true
}

// GetIngot retrieves full ingot (only if < 30 days old)
//
// Parameters:
//   - ingotID: The ingot identifier
//
// Returns:
//   - ingot: The full TokenTorqIngot
//   - found: true if full data available
func (ia *IngotArchive) GetIngot(ingotID string) (*models.TokenTorqIngot, bool) {
	ia.mu.RLock()
	defer ia.mu.RUnlock()

	entry, exists := ia.fullIngots[ingotID]
	if !exists || entry == nil {
		return nil, false
	}

	// Check if expired
	if time.Now().After(entry.ExpiresAt) {
		return nil, false
	}

	return entry.Ingot, true
}

// CleanupExpired removes expired full ingot data (keeps hashes)
//
// Should be called periodically (e.g., daily) to reclaim memory
//
// Returns:
//   - count: Number of expired ingots cleaned up
func (ia *IngotArchive) CleanupExpired() int {
	ia.mu.Lock()
	defer ia.mu.Unlock()

	now := time.Now()
	cleaned := 0

	for ingotID, entry := range ia.fullIngots {
		if now.After(entry.ExpiresAt) {
			delete(ia.fullIngots, ingotID)
			cleaned++
		}
	}

	if cleaned > 0 {
		ia.logger.Info("cleaned up expired ingots",
			"count", cleaned,
			"remaining_full_ingots", len(ia.fullIngots),
			"total_hash_entries", len(ia.jtuHashes))
	}

	return cleaned
}

// Stats returns archive statistics
func (ia *IngotArchive) Stats() IngotArchiveStats {
	ia.mu.RLock()
	defer ia.mu.RUnlock()

	totalHashes := 0
	for _, hashMap := range ia.jtuHashes {
		totalHashes += len(hashMap)
	}

	return IngotArchiveStats{
		TotalIngotsWithHashes: len(ia.jtuHashes),
		TotalJTUHashes:        totalHashes,
		FullIngotsAvailable:   len(ia.fullIngots),
	}
}

// IngotArchiveStats represents archive statistics
type IngotArchiveStats struct {
	TotalIngotsWithHashes int `json:"total_ingots_with_hashes"` // Permanent storage
	TotalJTUHashes        int `json:"total_jtu_hashes"`         // All hashes stored
	FullIngotsAvailable   int `json:"full_ingots_available"`    // Still within 30-day window
}

// MarshalJSON implements json.Marshaler for stats logging
func (s IngotArchiveStats) MarshalJSON() ([]byte, error) {
	return json.Marshal(map[string]interface{}{
		"total_ingots_with_hashes": s.TotalIngotsWithHashes,
		"total_jtu_hashes":         s.TotalJTUHashes,
		"full_ingots_available":    s.FullIngotsAvailable,
	})
}

// StartCleanupRoutine starts a background goroutine to clean up expired ingots
//
// Parameters:
//   - interval: How often to run cleanup (e.g., 24 hours)
//
// Returns:
//   - stop: Channel to signal cleanup routine to stop
func (ia *IngotArchive) StartCleanupRoutine(interval time.Duration) chan struct{} {
	stop := make(chan struct{})

	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				ia.CleanupExpired()
			case <-stop:
				ia.logger.Info("ingot archive cleanup routine stopped")
				return
			}
		}
	}()

	ia.logger.Info("ingot archive cleanup routine started",
		"interval", interval)

	return stop
}
