package mint

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewVerificationHandler tests handler creation
func TestNewVerificationHandler(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	assert.NotNil(t, handler)
	assert.NotNil(t, handler.proofCache)
	assert.NotNil(t, handler.signatureArchive)
	assert.NotNil(t, handler.logger)
	assert.NotNil(t, handler.metrics)
	assert.NotNil(t, handler.server)
	assert.Equal(t, ":8081", handler.server.Addr)
}

// TestVerificationHandler_Health tests the health check endpoint
func TestVerificationHandler_Health(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	// Create test request
	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()

	// Call handler
	handler.handleHealth(w, req)

	// Verify response
	assert.Equal(t, http.StatusOK, w.Code)
	assert.Equal(t, "application/json", w.Header().Get("Content-Type"))

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "healthy", response["status"])
	assert.Equal(t, float64(0), response["cache_size"]) // Empty cache
}

// TestVerificationHandler_ProofRequest_Success tests successful proof generation
func TestVerificationHandler_ProofRequest_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()

	// Populate cache with test data
	hashes := make([]string, 1000)
	hashEntries := make([]*models.IngotHashEntry, 1000)
	for i := 0; i < 1000; i++ {
		hashes[i] = generateTestHash(i)
		hashEntries[i] = &models.IngotHashEntry{
			BranchHash:  hashes[i],
			ContractIDs: []string{"test-contract"},
			DiggerIDs:   []string{"test-digger"},
			RefineryID:  "test-refinery",
		}
	}

	root, height, treeNodes, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  root,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		HashEntries: hashEntries, // Required for GetProof() bounds checking
	}

	unitID := "RT-test-unit-001"
	proofCache.Store(unitID, merkleResult)
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	// Create proof request
	reqBody := map[string]interface{}{
		"unit_id":     unitID,
		"ingot_index": 42,
	}
	reqJSON, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/proof", bytes.NewReader(reqJSON))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Call handler
	handler.handleProofRequest(w, req)

	// Verify response
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, unitID, response["unit_id"])
	assert.Equal(t, float64(42), response["ingot_index"])
	assert.Equal(t, root, response["merkle_root"])
	assert.Equal(t, float64(height), response["tree_height"])
	assert.True(t, response["verified"].(bool))

	proof := response["proof"].([]interface{})
	assert.Equal(t, height, len(proof)) // Proof length = tree height

	t.Logf("Proof generated: %d hashes, verified: %v", len(proof), response["verified"])
}

// TestVerificationHandler_ProofRequest_UnitNotFound tests 404 response
func TestVerificationHandler_ProofRequest_UnitNotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	// Create proof request for non-existent unit
	reqBody := map[string]interface{}{
		"unit_id":     "RT-nonexistent",
		"ingot_index": 42,
	}
	reqJSON, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/proof", bytes.NewReader(reqJSON))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.handleProofRequest(w, req)

	assert.Equal(t, http.StatusNotFound, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "RT-nonexistent", response["unit_id"])
	assert.Contains(t, response["error"], "not found")
}

// TestVerificationHandler_ProofRequest_InvalidIndex tests invalid ingot index
func TestVerificationHandler_ProofRequest_InvalidIndex(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	testCases := []struct {
		name        string
		ingotIndex  int
		expectedMsg string
	}{
		{"negative index", -1, "ingot_index must be 0-999"},
		{"index too large", 1000, "ingot_index must be 0-999"},
		{"index far too large", 9999, "ingot_index must be 0-999"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			reqBody := map[string]interface{}{
				"unit_id":     "RT-test",
				"ingot_index": tc.ingotIndex,
			}
			reqJSON, _ := json.Marshal(reqBody)

			req := httptest.NewRequest(http.MethodPost, "/verify/proof", bytes.NewReader(reqJSON))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			handler.handleProofRequest(w, req)

			assert.Equal(t, http.StatusBadRequest, w.Code)

			var response map[string]interface{}
			err := json.NewDecoder(w.Body).Decode(&response)
			require.NoError(t, err)

			assert.Contains(t, response["error"], tc.expectedMsg)
		})
	}
}

// TestVerificationHandler_ProofRequest_InvalidJSON tests malformed JSON
func TestVerificationHandler_ProofRequest_InvalidJSON(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	req := httptest.NewRequest(http.MethodPost, "/verify/proof", bytes.NewReader([]byte("invalid json")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.handleProofRequest(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "invalid JSON")
}

// TestVerificationHandler_ProofRequest_MissingUnitID tests missing unit_id
func TestVerificationHandler_ProofRequest_MissingUnitID(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	reqBody := map[string]interface{}{
		"ingot_index": 42,
		// unit_id missing
	}
	reqJSON, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/proof", bytes.NewReader(reqJSON))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.handleProofRequest(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "unit_id is required")
}

// TestVerificationHandler_ProofRequest_MethodNotAllowed tests GET instead of POST
func TestVerificationHandler_ProofRequest_MethodNotAllowed(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	req := httptest.NewRequest(http.MethodGet, "/verify/proof", nil)
	w := httptest.NewRecorder()

	handler.handleProofRequest(w, req)

	assert.Equal(t, http.StatusMethodNotAllowed, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "only POST method allowed")
}

// TestVerificationHandler_JTULookup_NotImplemented tests JTU lookup placeholder
func TestVerificationHandler_JTULookup_NotImplemented(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	ingotHash := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	req := httptest.NewRequest(http.MethodGet, "/verify/jtu/"+ingotHash, nil)
	w := httptest.NewRecorder()

	handler.handleJTULookup(w, req)

	assert.Equal(t, http.StatusNotImplemented, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, ingotHash, response["ingot_hash"])
	assert.False(t, response["found"].(bool))
	assert.Contains(t, response["error"], "not yet implemented")
}

// TestVerificationHandler_JTULookup_InvalidHash tests invalid hash format
func TestVerificationHandler_JTULookup_InvalidHash(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	testCases := []struct {
		name string
		hash string
	}{
		{"empty hash", ""},
		{"too short", "abc123"},
		{"too long", "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccddextra"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/verify/jtu/"+tc.hash, nil)
			w := httptest.NewRecorder()

			handler.handleJTULookup(w, req)

			assert.Equal(t, http.StatusBadRequest, w.Code)

			var response map[string]interface{}
			err := json.NewDecoder(w.Body).Decode(&response)
			require.NoError(t, err)

			assert.Contains(t, response["error"], "invalid ingot hash format")
		})
	}
}

// TestVerificationHandler_Shutdown tests graceful shutdown
func TestVerificationHandler_Shutdown(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":18081", logger, metrics) // Use unique port

	// Start server in background
	go handler.Start()

	// Give server time to start
	// time.Sleep(100 * time.Millisecond)

	// Shutdown
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	err := handler.Shutdown(ctx)
	assert.NoError(t, err)
}
