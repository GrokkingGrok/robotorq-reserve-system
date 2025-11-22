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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	assert.NotNil(t, handler)
	assert.NotNil(t, handler.proofCache)
	assert.NotNil(t, handler.signatureArchive)
	assert.NotNil(t, handler.logger)
	assert.NotNil(t, handler.metrics)
	assert.NotNil(t, handler.server)
	assert.Equal(t, ":8080", handler.server.Addr)
}

// TestVerificationHandler_Health tests the health check endpoint
func TestVerificationHandler_Health(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

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
func TestVerificationHandler_JTULookup_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

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
		HashEntries: hashEntries,
	}

	unitID := "RT-test-unit-001"
	proofCache.Store(unitID, merkleResult)

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	// Test lookup for ingot at index 42
	testHash := hashes[42]
	req := httptest.NewRequest(http.MethodGet, "/verify/jtu/"+testHash, nil)
	w := httptest.NewRecorder()

	handler.handleJTULookup(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, testHash, response["ingot_hash"])
	assert.True(t, response["found"].(bool))
	assert.Equal(t, unitID, response["unit_id"])
	assert.Equal(t, float64(42), response["ingot_index"])
	assert.Equal(t, root, response["merkle_root"])
}

// TestVerificationHandler_JTULookup_NotFound tests JTU lookup with hash not in cache
func TestVerificationHandler_JTULookup_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8081", logger, metrics)

	ingotHash := "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd"
	req := httptest.NewRequest(http.MethodGet, "/verify/jtu/"+ingotHash, nil)
	w := httptest.NewRecorder()

	handler.handleJTULookup(w, req)

	// Design choice: not-found returns 200 with found=false (lightweight lookup)
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, ingotHash, response["ingot_hash"])
	assert.False(t, response["found"].(bool))
	// No error field expected in this mode
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

// ============================================================================
// Certificate Verification Tests (NEW)
// ============================================================================

// TestHandleCertificateVerification_ValidCertificate tests certificate verification with valid merkle_root
func TestHandleCertificateVerification_ValidCertificate(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	// Create proof cache with test data
	proofCache := NewProofCache()
	testUnitID := "RT-20251119-001"
	testMerkleRoot := "abc123def456"

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

	_, height, treeNodes, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  testMerkleRoot,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		HashEntries: hashEntries,
	}
	proofCache.Store(testUnitID, merkleResult)

	signatureArchive := NewSignatureArchive()
	signatureArchive.Store(&SignatureRecord{
		UnitID:     testUnitID,
		MerkleRoot: testMerkleRoot,
		MintedAt:   time.Now().Format(time.RFC3339),
	})

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create request
	reqBody := map[string]string{
		"merkle_root": testMerkleRoot,
		"unit_id":     testUnitID,
	}
	bodyBytes, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/certificate", bytes.NewReader(bodyBytes))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.True(t, response["valid"].(bool), "Expected valid=true")
	assert.Equal(t, testUnitID, response["unit_id"])
	assert.Equal(t, testMerkleRoot, response["merkle_root"])
	assert.Equal(t, float64(1000), response["ingot_count"])
}

// TestHandleCertificateVerification_ValidCertificate_NoUnitID tests certificate verification without unit_id
func TestHandleCertificateVerification_ValidCertificate_NoUnitID(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	proofCache := NewProofCache()
	testUnitID := "RT-20251119-002"
	testMerkleRoot := "unique-merkle-root-xyz"

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

	_, height, treeNodes, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  testMerkleRoot,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		HashEntries: hashEntries,
	}
	proofCache.Store(testUnitID, merkleResult)

	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create request without unit_id (should trigger FindByMerkleRoot)
	reqBody := map[string]string{
		"merkle_root": testMerkleRoot,
	}
	bodyBytes, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/certificate", bytes.NewReader(bodyBytes))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.True(t, response["valid"].(bool), "Expected valid=true")
	assert.Equal(t, testUnitID, response["unit_id"], "Should find unit_id via FindByMerkleRoot")
	assert.Equal(t, testMerkleRoot, response["merkle_root"])
}

// TestHandleCertificateVerification_InvalidCertificate tests certificate verification with unknown merkle_root
func TestHandleCertificateVerification_InvalidCertificate(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create request with unknown merkle_root
	reqBody := map[string]string{
		"merkle_root": "nonexistent-merkle-root-fraud-attempt",
	}
	bodyBytes, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/certificate", bytes.NewReader(bodyBytes))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusOK, w.Code, "Should return 200 with valid=false")

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.False(t, response["valid"].(bool), "Expected valid=false for unknown certificate")
	assert.Contains(t, response["error"], "certificate not found")
}

// TestHandleCertificateVerification_MerkleRootMismatch tests unit_id exists but merkle_root doesn't match
func TestHandleCertificateVerification_MerkleRootMismatch(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	proofCache := NewProofCache()
	testUnitID := "RT-20251119-003"
	correctMerkleRoot := "correct-merkle-root"
	wrongMerkleRoot := "wrong-merkle-root"

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

	_, height, treeNodes, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  correctMerkleRoot,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		HashEntries: hashEntries,
	}
	proofCache.Store(testUnitID, merkleResult)

	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create request with wrong merkle_root
	reqBody := map[string]string{
		"merkle_root": wrongMerkleRoot,
		"unit_id":     testUnitID,
	}
	bodyBytes, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/certificate", bytes.NewReader(bodyBytes))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.False(t, response["valid"].(bool), "Expected valid=false for merkle_root mismatch")
	assert.Contains(t, response["error"], "does not match")
}

// TestHandleCertificateVerification_MissingMerkleRoot tests validation of required fields
func TestHandleCertificateVerification_MissingMerkleRoot(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create request with missing merkle_root
	reqBody := map[string]string{
		"unit_id": "RT-20251119-001",
	}
	bodyBytes, _ := json.Marshal(reqBody)

	req := httptest.NewRequest(http.MethodPost, "/verify/certificate", bytes.NewReader(bodyBytes))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusBadRequest, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "merkle_root is required")
}

// TestHandleCertificateVerification_WrongMethod tests method validation
func TestHandleCertificateVerification_WrongMethod(t *testing.T) {
	// Setup
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)

	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(
		proofCache,
		signatureArchive,
		"test-public-key",
		":8080",
		logger,
		metrics,
	)

	// Create GET request (should be POST)
	req := httptest.NewRequest(http.MethodGet, "/verify/certificate", nil)
	w := httptest.NewRecorder()

	// Execute
	handler.handleCertificateVerification(w, req)

	// Assert
	assert.Equal(t, http.StatusMethodNotAllowed, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "only POST method allowed")
}

// TestFindByMerkleRoot tests ProofCache reverse lookup
func TestFindByMerkleRoot(t *testing.T) {
	// Setup
	proofCache := NewProofCache()

	testCases := []struct {
		unitID     string
		merkleRoot string
	}{
		{"RT-20251119-001", "merkle-root-001"},
		{"RT-20251119-002", "merkle-root-002"},
		{"RT-20251119-003", "merkle-root-003"},
	}

	// Store test data
	for _, tc := range testCases {
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

		_, height, treeNodes, err := buildMerkleTree(hashes)
		require.NoError(t, err)

		merkleResult := &Level2MerkleResult{
			MerkleRoot:  tc.merkleRoot,
			TreeHeight:  height,
			TreeNodes:   treeNodes,
			HashEntries: hashEntries,
		}
		proofCache.Store(tc.unitID, merkleResult)
	}

	// Test lookups
	for _, tc := range testCases {
		foundUnitID := proofCache.FindByMerkleRoot(tc.merkleRoot)
		assert.Equal(t, tc.unitID, foundUnitID, "FindByMerkleRoot should find correct unit_id")
	}

	// Test non-existent merkle root
	notFound := proofCache.FindByMerkleRoot("nonexistent-merkle-root")
	assert.Empty(t, notFound, "Should return empty string for non-existent merkle root")
}

// TestFindByMerkleRoot_EmptyCache tests lookup in empty cache
func TestFindByMerkleRoot_EmptyCache(t *testing.T) {
	proofCache := NewProofCache()

	result := proofCache.FindByMerkleRoot("any-merkle-root")
	assert.Empty(t, result, "Should return empty string for empty cache")
}

// TestFindByMerkleRoot_ConcurrentAccess tests thread safety
func TestFindByMerkleRoot_ConcurrentAccess(t *testing.T) {
	proofCache := NewProofCache()

	// Store initial data
	testUnitID := "RT-20251119-concurrent"
	testMerkleRoot := "merkle-root-concurrent-test"

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

	_, height, treeNodes, err := buildMerkleTree(hashes)
	require.NoError(t, err)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  testMerkleRoot,
		TreeHeight:  height,
		TreeNodes:   treeNodes,
		HashEntries: hashEntries,
	}
	proofCache.Store(testUnitID, merkleResult)

	// Concurrent reads
	done := make(chan bool)
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 100; j++ {
				unitID := proofCache.FindByMerkleRoot(testMerkleRoot)
				assert.Equal(t, testUnitID, unitID, "Concurrent access should return correct unit_id")
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}
}
