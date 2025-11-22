package mint

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestVerificationHandler_SignatureRequest_Success tests successful signature retrieval
func TestVerificationHandler_SignatureRequest_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	// Add test signature to archive
	testRecord := &SignatureRecord{
		UnitID:     "RT-test-unit-001",
		Signature:  "abcd1234567890abcd1234567890abcd1234567890abcd1234567890abcd1234567890abcd1234",
		PublicKey:  "ef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
		MerkleRoot: "aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd",
		MintedAt:   "2025-11-17T10:00:00Z",
		SignedAt:   time.Now().UTC().Format(time.RFC3339),
	}
	err := signatureArchive.Store(testRecord)
	require.NoError(t, err)

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	// Create signature request
	req := httptest.NewRequest(http.MethodGet, "/verify/signature/RT-test-unit-001", nil)
	w := httptest.NewRecorder()

	// Call handler
	handler.handleSignatureRequest(w, req)

	// Verify response
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "RT-test-unit-001", response["unit_id"])
	assert.Equal(t, testRecord.Signature, response["signature"])
	assert.Equal(t, testRecord.PublicKey, response["public_key"])
	assert.Equal(t, testRecord.MerkleRoot, response["merkle_root"])
	assert.Equal(t, testRecord.MintedAt, response["minted_at"])
	assert.NotEmpty(t, response["signed_at"])

	t.Logf("Signature retrieved: %s", response["signature"])
}

// TestVerificationHandler_SignatureRequest_NotFound tests signature not found
func TestVerificationHandler_SignatureRequest_NotFound(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	// Request non-existent signature
	req := httptest.NewRequest(http.MethodGet, "/verify/signature/RT-nonexistent-unit", nil)
	w := httptest.NewRecorder()

	handler.handleSignatureRequest(w, req)

	assert.Equal(t, http.StatusNotFound, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "signature not found")
	assert.Equal(t, "RT-nonexistent-unit", response["unit_id"])
}

// TestVerificationHandler_SignatureRequest_MissingUnitID tests empty unit ID
func TestVerificationHandler_SignatureRequest_MissingUnitID(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	// Request with empty unit_id
	req := httptest.NewRequest(http.MethodGet, "/verify/signature/", nil)
	w := httptest.NewRecorder()

	handler.handleSignatureRequest(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Contains(t, response["error"], "unit_id is required")
}

// TestVerificationHandler_Health_IncludesSignatureCount tests health endpoint reports signature count
func TestVerificationHandler_Health_IncludesSignatureCount(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	// Add test signatures
	signatureArchive.Store(&SignatureRecord{
		UnitID:     "RT-test-1",
		Signature:  "sig1",
		PublicKey:  "pub1",
		MerkleRoot: "root1",
		MintedAt:   "2025-11-17T10:00:00Z",
		SignedAt:   time.Now().UTC().Format(time.RFC3339),
	})
	signatureArchive.Store(&SignatureRecord{
		UnitID:     "RT-test-2",
		Signature:  "sig2",
		PublicKey:  "pub2",
		MerkleRoot: "root2",
		MintedAt:   "2025-11-17T10:01:00Z",
		SignedAt:   time.Now().UTC().Format(time.RFC3339),
	})

	handler := NewVerificationHandler(proofCache, signatureArchive, "test-public-key", ":8080", logger, metrics)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()

	handler.handleHealth(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "healthy", response["status"])
	assert.Equal(t, float64(0), response["cache_size"])      // ProofCache empty
	assert.Equal(t, float64(2), response["signature_count"]) // 2 signatures stored

	t.Logf("Health response: %+v", response)
}
