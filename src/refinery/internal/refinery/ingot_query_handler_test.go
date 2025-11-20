// ingot_query_handler_test.go - Unit tests for IngotQueryHandler
package refinery

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"b2b/refinery/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestNewIngotQueryHandler(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	assert.NotNil(t, handler)
	assert.NotNil(t, handler.archive)
	assert.NotNil(t, handler.logger)
}

func TestIngotQueryHandler_HandleJTUHash(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	// Store test ingot
	ingot := createTestIngot("hash-test-ingot", 100)
	require.NoError(t, archive.Store(ingot))

	// Test valid hash request
	req := httptest.NewRequest(http.MethodGet, "/ingot/hash-test-ingot/jtu/42/hash", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "hash-test-ingot", response["ingot_id"])
	assert.Equal(t, float64(42), response["jtu_index"])
	assert.True(t, response["found"].(bool))
	assert.Equal(t, ingot.Units[42].Hash, response["hash"])
}

func TestIngotQueryHandler_HandleJTUHash_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	req := httptest.NewRequest(http.MethodGet, "/ingot/non-existent/jtu/0/hash", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "non-existent", response["ingot_id"])
	assert.False(t, response["found"].(bool))
}

func TestIngotQueryHandler_HandleJTU(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	// Store test ingot
	ingot := createTestIngot("jtu-test-ingot", 100)
	require.NoError(t, archive.Store(ingot))

	// Test valid JTU request
	req := httptest.NewRequest(http.MethodGet, "/ingot/jtu-test-ingot/jtu/42", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "jtu-test-ingot", response["ingot_id"])
	assert.Equal(t, float64(42), response["jtu_index"])
	assert.True(t, response["found"].(bool))
	assert.NotNil(t, response["unit"])

	// Verify unit data
	unit := response["unit"].(map[string]interface{})
	assert.Equal(t, ingot.Units[42].TokenID, unit["token_id"])
	assert.Equal(t, ingot.Units[42].Hash, unit["hash"])
}

func TestIngotQueryHandler_HandleJTU_Expired(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	// Store and expire ingot
	ingot := createTestIngot("expired-ingot", 10)
	require.NoError(t, archive.Store(ingot))

	archive.mu.Lock()
	archive.fullIngots["expired-ingot"].ExpiresAt = time.Now().Add(-1 * time.Hour)
	archive.mu.Unlock()

	// Request expired JTU
	req := httptest.NewRequest(http.MethodGet, "/ingot/expired-ingot/jtu/5", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.False(t, response["found"].(bool))
	assert.Contains(t, response["reason"], "expired")
	assert.NotNil(t, response["hash"]) // Hash still provided
}

func TestIngotQueryHandler_HandleIngot(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	// Store test ingot
	ingot := createTestIngot("full-ingot-test", 50)
	require.NoError(t, archive.Store(ingot))

	// Request full ingot
	req := httptest.NewRequest(http.MethodGet, "/ingot/full-ingot-test", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "full-ingot-test", response["ingot_id"])
	assert.True(t, response["found"].(bool))
	assert.NotNil(t, response["ingot"])

	// Verify ingot structure
	ingotData := response["ingot"].(map[string]interface{})
	assert.Equal(t, "full-ingot-test", ingotData["ingot_id"])
	assert.NotNil(t, ingotData["units"])
}

func TestIngotQueryHandler_HandleIngot_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	req := httptest.NewRequest(http.MethodGet, "/ingot/non-existent", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.False(t, response["found"].(bool))
	assert.Contains(t, response["reason"], "expired or not found")
}

func TestIngotQueryHandler_HandleStats(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	// Store multiple ingots
	require.NoError(t, archive.Store(createTestIngot("stats-1", 100)))
	require.NoError(t, archive.Store(createTestIngot("stats-2", 200)))

	req := httptest.NewRequest(http.MethodGet, "/archive/stats", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var stats IngotArchiveStats
	err := json.NewDecoder(w.Body).Decode(&stats)
	require.NoError(t, err)

	assert.Equal(t, 2, stats.TotalIngotsWithHashes)
	assert.Equal(t, 300, stats.TotalJTUHashes) // 100 + 200
	assert.Equal(t, 2, stats.FullIngotsAvailable)
}

func TestIngotQueryHandler_InvalidPath(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	tests := []struct {
		name string
		path string
	}{
		{"empty path", "/"},
		{"invalid endpoint", "/invalid"},
		{"missing ingot id", "/ingot/"},
		{"missing jtu index", "/ingot/test/jtu/"},
		{"invalid jtu index", "/ingot/test/jtu/abc"},
		{"jtu index too high", "/ingot/test/jtu/9999"},
		{"jtu index negative", "/ingot/test/jtu/-1"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, tt.path, nil)
			w := httptest.NewRecorder()

			handler.ServeHTTP(w, req)

			// Should return error (400 or 404)
			assert.True(t, w.Code == http.StatusBadRequest || w.Code == http.StatusNotFound)

			var response map[string]interface{}
			err := json.NewDecoder(w.Body).Decode(&response)
			require.NoError(t, err)
			assert.NotEmpty(t, response["error"])
		})
	}
}

func TestIngotQueryHandler_MethodNotAllowed(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	paths := []string{
		"/ingot/test/jtu/0/hash",
		"/ingot/test/jtu/0",
		"/ingot/test",
		"/archive/stats",
	}

	for _, path := range paths {
		t.Run(path, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, path, nil)
			w := httptest.NewRecorder()

			handler.ServeHTTP(w, req)

			assert.Equal(t, http.StatusMethodNotAllowed, w.Code)

			var response map[string]interface{}
			err := json.NewDecoder(w.Body).Decode(&response)
			require.NoError(t, err)
			assert.Contains(t, response["error"], "only GET allowed")
		})
	}
}

func TestIngotQueryHandler_CORS(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	req := httptest.NewRequest(http.MethodGet, "/archive/stats", nil)
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	assert.Equal(t, "*", w.Header().Get("Access-Control-Allow-Origin"))
	assert.Equal(t, "application/json", w.Header().Get("Content-Type"))
}

func TestIngotQueryHandler_ParseJTUPath(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	archive := NewIngotArchive(logger)
	handler := NewIngotQueryHandler(archive, logger)

	tests := []struct {
		name        string
		path        string
		wantIngotID string
		wantIndex   int
		wantErr     bool
	}{
		{
			name:        "valid path",
			path:        "/ingot/test-123/jtu/42",
			wantIngotID: "test-123",
			wantIndex:   42,
			wantErr:     false,
		},
		{
			name:        "valid path with hash suffix",
			path:        "/ingot/test-456/jtu/99/hash",
			wantIngotID: "test-456",
			wantIndex:   99,
			wantErr:     false,
		},
		{
			name:    "invalid format",
			path:    "/invalid",
			wantErr: true,
		},
		{
			name:    "missing ingot id",
			path:    "/ingot//jtu/0",
			wantErr: true,
		},
		{
			name:    "invalid index",
			path:    "/ingot/test/jtu/abc",
			wantErr: true,
		},
		{
			name:    "index too high",
			path:    "/ingot/test/jtu/3600",
			wantErr: true,
		},
		{
			name:    "negative index",
			path:    "/ingot/test/jtu/-1",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ingotID, jtuIndex, err := handler.parseJTUPath(tt.path)

			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.wantIngotID, ingotID)
				assert.Equal(t, tt.wantIndex, jtuIndex)
			}
		})
	}
}
