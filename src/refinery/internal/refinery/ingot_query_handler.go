// ingot_query_handler.go - HTTP API for querying ingot archive
package refinery

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"strconv"
	"strings"
)

// IngotQueryHandler provides HTTP endpoints for querying the ingot archive
//
// Endpoints:
//   - GET /ingot/:ingot_id/jtu/:index/hash  - Get JTU hash (permanent, always available)
//   - GET /ingot/:ingot_id/jtu/:index       - Get full JTU details (only if < 30 days)
//   - GET /ingot/:ingot_id                  - Get full ingot (only if < 30 days)
//   - GET /archive/stats                    - Get archive statistics
type IngotQueryHandler struct {
	archive *IngotArchive
	logger  *slog.Logger
}

// NewIngotQueryHandler creates a new query handler
func NewIngotQueryHandler(archive *IngotArchive, logger *slog.Logger) *IngotQueryHandler {
	return &IngotQueryHandler{
		archive: archive,
		logger:  logger,
	}
}

// ServeHTTP implements http.Handler interface
func (h *IngotQueryHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Enable CORS
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Content-Type", "application/json")

	path := r.URL.Path

	switch {
	case strings.HasPrefix(path, "/ingot/") && strings.Contains(path, "/jtu/") && strings.HasSuffix(path, "/hash"):
		h.handleJTUHash(w, r)
	case strings.HasPrefix(path, "/ingot/") && strings.Contains(path, "/jtu/"):
		h.handleJTU(w, r)
	case strings.HasPrefix(path, "/ingot/"):
		h.handleIngot(w, r)
	case path == "/archive/stats":
		h.handleStats(w, r)
	default:
		h.respondError(w, http.StatusNotFound, "unknown endpoint")
	}
}

// handleJTUHash handles GET /ingot/:ingot_id/jtu/:index/hash
//
// Returns just the hash (available forever)
//
// Response (200):
//
//	{
//	  "ingot_id": "550e8400-...",
//	  "jtu_index": 42,
//	  "hash": "abc123...",
//	  "found": true
//	}
func (h *IngotQueryHandler) handleJTUHash(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		h.respondError(w, http.StatusMethodNotAllowed, "only GET allowed")
		return
	}

	ingotID, jtuIndex, err := h.parseJTUPath(r.URL.Path)
	if err != nil {
		h.respondError(w, http.StatusBadRequest, err.Error())
		return
	}

	hash, found := h.archive.GetJTUHash(ingotID, jtuIndex)

	if !found {
		h.respondJSON(w, http.StatusOK, map[string]interface{}{
			"ingot_id":  ingotID,
			"jtu_index": jtuIndex,
			"found":     false,
		})
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"ingot_id":  ingotID,
		"jtu_index": jtuIndex,
		"hash":      hash,
		"found":     true,
	})
}

// handleJTU handles GET /ingot/:ingot_id/jtu/:index
//
// Returns full JTU details (only if < 30 days old)
//
// Response (200):
//
//	{
//	  "ingot_id": "550e8400-...",
//	  "jtu_index": 42,
//	  "unit": {
//	    "token_id": "contract-1-42",
//	    "joules_consumed": 15.2,
//	    "robo_stake_paid": 0.01,
//	    ...
//	  },
//	  "found": true
//	}
//
// Response (404 - expired):
//
//	{
//	  "ingot_id": "550e8400-...",
//	  "jtu_index": 42,
//	  "found": false,
//	  "reason": "ingot data expired (>30 days), use /hash endpoint"
//	}
func (h *IngotQueryHandler) handleJTU(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		h.respondError(w, http.StatusMethodNotAllowed, "only GET allowed")
		return
	}

	ingotID, jtuIndex, err := h.parseJTUPath(r.URL.Path)
	if err != nil {
		h.respondError(w, http.StatusBadRequest, err.Error())
		return
	}

	unit, found := h.archive.GetJTU(ingotID, jtuIndex)

	if !found {
		// Check if hash exists (means ingot is old but still verifiable)
		if hash, hashFound := h.archive.GetJTUHash(ingotID, jtuIndex); hashFound {
			h.respondJSON(w, http.StatusOK, map[string]interface{}{
				"ingot_id":  ingotID,
				"jtu_index": jtuIndex,
				"found":     false,
				"reason":    "ingot data expired (>30 days), use /hash endpoint",
				"hash":      hash, // Still provide hash for verification
			})
		} else {
			h.respondJSON(w, http.StatusOK, map[string]interface{}{
				"ingot_id":  ingotID,
				"jtu_index": jtuIndex,
				"found":     false,
				"reason":    "ingot not found in archive",
			})
		}
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"ingot_id":  ingotID,
		"jtu_index": jtuIndex,
		"unit":      unit,
		"found":     true,
	})
}

// handleIngot handles GET /ingot/:ingot_id
//
// Returns full ingot with all 3600 JTUs (only if < 30 days old)
func (h *IngotQueryHandler) handleIngot(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		h.respondError(w, http.StatusMethodNotAllowed, "only GET allowed")
		return
	}

	// Extract ingot_id from path: /ingot/{ingot_id}
	parts := strings.Split(strings.TrimPrefix(r.URL.Path, "/ingot/"), "/")
	if len(parts) == 0 || parts[0] == "" {
		h.respondError(w, http.StatusBadRequest, "missing ingot_id")
		return
	}

	ingotID := parts[0]

	ingot, found := h.archive.GetIngot(ingotID)

	if !found {
		h.respondJSON(w, http.StatusOK, map[string]interface{}{
			"ingot_id": ingotID,
			"found":    false,
			"reason":   "ingot data expired (>30 days) or not found",
		})
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"ingot_id": ingotID,
		"ingot":    ingot,
		"found":    true,
	})
}

// handleStats handles GET /archive/stats
func (h *IngotQueryHandler) handleStats(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		h.respondError(w, http.StatusMethodNotAllowed, "only GET allowed")
		return
	}

	stats := h.archive.Stats()
	h.respondJSON(w, http.StatusOK, stats)
}

// parseJTUPath parses /ingot/{ingot_id}/jtu/{index}[/hash]
func (h *IngotQueryHandler) parseJTUPath(path string) (ingotID string, jtuIndex int, err error) {
	// Remove /hash suffix if present
	path = strings.TrimSuffix(path, "/hash")

	// Expected: /ingot/{ingot_id}/jtu/{index}
	parts := strings.Split(path, "/")
	if len(parts) < 5 {
		return "", 0, fmt.Errorf("invalid path format, expected /ingot/{ingot_id}/jtu/{index}")
	}

	ingotID = parts[2]
	if ingotID == "" {
		return "", 0, fmt.Errorf("missing ingot_id")
	}

	indexStr := parts[4]
	jtuIndex, err = strconv.Atoi(indexStr)
	if err != nil {
		return "", 0, fmt.Errorf("invalid jtu_index: %s", indexStr)
	}

	if jtuIndex < 0 || jtuIndex >= 3600 {
		return "", 0, fmt.Errorf("jtu_index must be 0-3599, got %d", jtuIndex)
	}

	return ingotID, jtuIndex, nil
}

// respondJSON sends a JSON response
func (h *IngotQueryHandler) respondJSON(w http.ResponseWriter, status int, data interface{}) {
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(data); err != nil {
		h.logger.Error("failed to encode JSON response", "error", err)
	}
}

// respondError sends an error response
func (h *IngotQueryHandler) respondError(w http.ResponseWriter, status int, message string) {
	h.respondJSON(w, status, map[string]string{
		"error": message,
	})
}
