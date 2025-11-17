package mint

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestVerificationHandler_PublicKey_Success tests successful public key retrieval
func TestVerificationHandler_PublicKey_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()
	testPublicKey := "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab" // 64 hex chars = 32 bytes

	handler := NewVerificationHandler(proofCache, signatureArchive, testPublicKey, ":8081", logger, metrics)

	// Create request
	req := httptest.NewRequest(http.MethodGet, "/public-key", nil)
	w := httptest.NewRecorder()

	// Call handler
	handler.handlePublicKey(w, req)

	// Verify response
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "SPHINCS+-SHA2-128f-simple", response["algorithm"])
	assert.Equal(t, testPublicKey, response["public_key"])
	assert.Equal(t, float64(32), response["key_size_bytes"]) // JSON decodes numbers as float64
	assert.Equal(t, "Verify SPHINCS+ signatures on Phase3RoboTorqUnits", response["purpose"])

	t.Logf("Public key endpoint response: %+v", response)
}

// TestVerificationHandler_PublicKey_CORS tests CORS headers
func TestVerificationHandler_PublicKey_CORS(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()
	testPublicKey := "test-public-key-64-chars-hexadecimal-string-for-sphincs-plus-ok"

	handler := NewVerificationHandler(proofCache, signatureArchive, testPublicKey, ":8081", logger, metrics)

	req := httptest.NewRequest(http.MethodGet, "/public-key", nil)
	w := httptest.NewRecorder()

	handler.handlePublicKey(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	assert.Equal(t, "application/json", w.Header().Get("Content-Type"))
	assert.Equal(t, "*", w.Header().Get("Access-Control-Allow-Origin"))
}

// TestVerificationHandler_PublicKey_EmptyKey tests handler with empty public key
func TestVerificationHandler_PublicKey_EmptyKey(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()
	emptyPublicKey := ""

	handler := NewVerificationHandler(proofCache, signatureArchive, emptyPublicKey, ":8081", logger, metrics)

	req := httptest.NewRequest(http.MethodGet, "/public-key", nil)
	w := httptest.NewRecorder()

	handler.handlePublicKey(w, req)

	// Should still return 200, but with empty key (initialization issue, not runtime error)
	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err := json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, "", response["public_key"])
	assert.Equal(t, float64(0), response["key_size_bytes"])
}

// TestVerificationHandler_PublicKey_Integration tests public key from real SPHINCS+ signer
func TestVerificationHandler_PublicKey_Integration(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewVerificationMetrics(nil)
	proofCache := NewProofCache()
	signatureArchive := NewSignatureArchive()

	// Create Phase3Assembler to get real public key
	phase3Metrics := NewPhase3AssemblerMetrics(nil)
	ingotHashQueue, _ := NewIngotHashQueue(1000, 100, logger)
	level2Builder := NewLevel2MerkleBuilder(ingotHashQueue, logger)

	assembler, err := NewPhase3RoboTorqUnitAssembler(logger, phase3Metrics, level2Builder, 10)
	require.NoError(t, err)

	realPublicKey := assembler.GetPublicKey()
	require.NotEmpty(t, realPublicKey, "Phase3Assembler should generate a public key")

	handler := NewVerificationHandler(proofCache, signatureArchive, realPublicKey, ":8081", logger, metrics)

	req := httptest.NewRequest(http.MethodGet, "/public-key", nil)
	w := httptest.NewRecorder()

	handler.handlePublicKey(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	err = json.NewDecoder(w.Body).Decode(&response)
	require.NoError(t, err)

	assert.Equal(t, realPublicKey, response["public_key"])
	assert.Equal(t, float64(32), response["key_size_bytes"]) // SPHINCS+-SHA2-128f-simple uses 32-byte public keys

	t.Logf("Real SPHINCS+ public key: %s (length: %d hex chars)", realPublicKey, len(realPublicKey))
}
