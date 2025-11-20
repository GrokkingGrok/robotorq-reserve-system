// Package mint provides the VerificationHandler component.
// VerificationHandler exposes HTTP endpoints for merkle proof verification.
package mint

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"strings"

	"github.com/prometheus/client_golang/prometheus"
)

// VerificationHandler handles HTTP requests for merkle proof verification
//
// Endpoints:
//   - GET  /verify/jtu/:hash      - Look up JouleTorqUnit by ingot hash
//   - POST /verify/proof          - Generate merkle proof for a Phase3 unit
//   - GET  /verify/signature/:id  - Get SPHINCS+ signature for a Phase3 unit
//   - GET  /public-key            - Get Mint's SPHINCS+ public key for verification
//   - GET  /health                - Health check endpoint
//
// Design:
//   - Read-only operations (no state modification)
//   - Uses ProofCache for O(log n) proof generation
//   - Uses SignatureArchive for SPHINCS+ signature retrieval
//   - Returns JSON responses with CORS headers
type VerificationHandler struct {
	proofCache       *ProofCache
	signatureArchive *SignatureArchive
	publicKey        string // SPHINCS+ public key (hex-encoded)
	logger           *slog.Logger
	metrics          *VerificationMetrics
	server           *http.Server
}

// VerificationMetrics tracks verification API metrics
type VerificationMetrics struct {
	JTULookupsTotal      prometheus.Counter
	JTULookupsFound      prometheus.Counter
	JTULookupsNotFound   prometheus.Counter
	ProofRequestsTotal   prometheus.Counter
	ProofRequestsSuccess prometheus.Counter
	ProofRequestsFailure prometheus.Counter
	InvalidRequestsTotal prometheus.Counter
	ResponseLatency      prometheus.Histogram
}

// NewVerificationMetrics creates Prometheus metrics for verification API
func NewVerificationMetrics(reg prometheus.Registerer) *VerificationMetrics {
	m := &VerificationMetrics{
		JTULookupsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_jtu_lookups_total",
			Help: "Total number of JTU hash lookup requests",
		}),
		JTULookupsFound: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_jtu_lookups_found_total",
			Help: "Number of successful JTU hash lookups",
		}),
		JTULookupsNotFound: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_jtu_lookups_not_found_total",
			Help: "Number of JTU hash lookups returning 404",
		}),
		ProofRequestsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_proof_requests_total",
			Help: "Total number of merkle proof requests",
		}),
		ProofRequestsSuccess: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_proof_requests_success_total",
			Help: "Number of successful proof generations",
		}),
		ProofRequestsFailure: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_proof_requests_failure_total",
			Help: "Number of failed proof generations",
		}),
		InvalidRequestsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_verification_invalid_requests_total",
			Help: "Total number of invalid requests (400 errors)",
		}),
		ResponseLatency: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_verification_response_latency_seconds",
			Help:    "Response latency for verification API requests",
			Buckets: []float64{0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0},
		}),
	}

	if reg != nil {
		reg.MustRegister(
			m.JTULookupsTotal,
			m.JTULookupsFound,
			m.JTULookupsNotFound,
			m.ProofRequestsTotal,
			m.ProofRequestsSuccess,
			m.ProofRequestsFailure,
			m.InvalidRequestsTotal,
			m.ResponseLatency,
		)
	}

	return m
}

// NewVerificationHandler creates a new verification API handler
//
// Parameters:
//   - proofCache: Cache of Level2MerkleResults for proof generation
//   - signatureArchive: Archive of SPHINCS+ signatures
//   - publicKey: Mint's SPHINCS+ public key (hex-encoded) for distribution
//   - port: HTTP port to listen on (e.g., ":8081")
//   - logger: Structured logger
//   - metrics: Prometheus metrics
//
// Returns:
//   - VerificationHandler ready to start
func NewVerificationHandler(
	proofCache *ProofCache,
	signatureArchive *SignatureArchive,
	publicKey string,
	port string,
	logger *slog.Logger,
	metrics *VerificationMetrics,
) *VerificationHandler {
	mux := http.NewServeMux()

	handler := &VerificationHandler{
		proofCache:       proofCache,
		signatureArchive: signatureArchive,
		publicKey:        publicKey,
		logger:           logger,
		metrics:          metrics,
		server: &http.Server{
			Addr:    port,
			Handler: mux,
		},
	}

	// Register routes
	mux.HandleFunc("/verify/jtu/", handler.handleJTULookup)
	mux.HandleFunc("/verify/proof", handler.handleProofRequest)
	mux.HandleFunc("/verify/signature/", handler.handleSignatureRequest)
	mux.HandleFunc("/verify/certificate", handler.handleCertificateVerification)
	mux.HandleFunc("/public-key", handler.handlePublicKey)
	mux.HandleFunc("/health", handler.handleHealth)

	return handler
}

// Start begins listening for HTTP requests
//
// This is a blocking call - run in a goroutine:
//
//	go handler.Start()
//
// Returns:
//   - Error if server fails to start
func (h *VerificationHandler) Start() error {
	h.logger.Info("verification API starting",
		"addr", h.server.Addr)

	if err := h.server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("verification API error: %w", err)
	}

	return nil
}

// Shutdown performs graceful shutdown of the HTTP server
//
// Parameters:
//   - ctx: Context with timeout for shutdown
//
// Returns:
//   - Error if shutdown fails
func (h *VerificationHandler) Shutdown(ctx context.Context) error {
	h.logger.Info("shutting down verification API")
	return h.server.Shutdown(ctx)
}

// handleJTULookup handles GET /verify/jtu/:hash
//
// Request:
//   - GET /verify/jtu/aabbccdd...
//
// Response (200):
//
//	{
//	  "ingot_hash": "aabbccdd...",
//	  "unit_id": "RT-20251117-123456.789012",
//	  "merkle_root": "xyz...",
//	  "found": true
//	}
//
// Response (404):
//
//	{
//	  "ingot_hash": "aabbccdd...",
//	  "found": false,
//	  "error": "ingot hash not found in any Phase3 unit"
//	}
func (h *VerificationHandler) handleJTULookup(w http.ResponseWriter, r *http.Request) {
	timer := prometheus.NewTimer(h.metrics.ResponseLatency)
	defer timer.ObserveDuration()

	h.metrics.JTULookupsTotal.Inc()

	// Extract hash from URL path: /verify/jtu/{hash}
	path := strings.TrimPrefix(r.URL.Path, "/verify/jtu/")
	ingotHash := strings.TrimSpace(path)

	if ingotHash == "" || len(ingotHash) != 64 {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, "invalid ingot hash format (expected 64-char hex)")
		return
	}

	h.logger.Debug("jtu lookup request",
		"ingot_hash", ingotHash)

	// Look up unit ID by ingot hash (reverse index)
	unitID, found := h.proofCache.LookupByIngotHash(ingotHash)

	if !found {
		h.metrics.JTULookupsNotFound.Inc()
		h.respondJSON(w, http.StatusOK, map[string]interface{}{
			"ingot_hash": ingotHash,
			"found":      false,
		})
		return
	}

	// Get the merkle result for this unit
	merkleResult := h.proofCache.Get(unitID)
	if merkleResult == nil {
		// This should never happen (index inconsistency)
		h.logger.Error("index inconsistency: unit ID found but merkle result missing",
			"unit_id", unitID,
			"ingot_hash", ingotHash)
		h.metrics.JTULookupsNotFound.Inc()
		h.respondError(w, http.StatusInternalServerError, "cache inconsistency detected")
		return
	}

	// Find the ingot index within the unit
	var ingotIndex int
	found = false
	for i, entry := range merkleResult.HashEntries {
		if entry.BranchHash == ingotHash {
			ingotIndex = i
			found = true
			break
		}
	}

	if !found {
		// Another inconsistency - should never happen
		h.logger.Error("index inconsistency: hash in index but not in hash entries",
			"unit_id", unitID,
			"ingot_hash", ingotHash)
		h.metrics.JTULookupsNotFound.Inc()
		h.respondError(w, http.StatusInternalServerError, "cache inconsistency detected")
		return
	}

	h.metrics.JTULookupsFound.Inc()
	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"ingot_hash":  ingotHash,
		"found":       true,
		"unit_id":     unitID,
		"ingot_index": ingotIndex,
		"merkle_root": merkleResult.MerkleRoot,
		"tree_height": merkleResult.TreeHeight,
	})
}

// handleProofRequest handles POST /verify/proof
//
// Request body:
//
//	{
//	  "unit_id": "RT-20251117-123456.789012",
//	  "ingot_index": 42
//	}
//
// Response (200):
//
//	{
//	  "unit_id": "RT-20251117-123456.789012",
//	  "ingot_index": 42,
//	  "merkle_root": "xyz...",
//	  "tree_height": 10,
//	  "proof": ["hash1", "hash2", ...],
//	  "verified": true
//	}
//
// Response (404):
//
//	{
//	  "unit_id": "RT-20251117-123456.789012",
//	  "error": "unit not found in proof cache"
//	}
func (h *VerificationHandler) handleProofRequest(w http.ResponseWriter, r *http.Request) {
	timer := prometheus.NewTimer(h.metrics.ResponseLatency)
	defer timer.ObserveDuration()

	h.metrics.ProofRequestsTotal.Inc()

	if r.Method != http.MethodPost {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusMethodNotAllowed, "only POST method allowed")
		return
	}

	// Parse request body
	var req struct {
		UnitID     string `json:"unit_id"`
		IngotIndex int    `json:"ingot_index"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, fmt.Sprintf("invalid JSON: %v", err))
		return
	}

	h.logger.Debug("proof request",
		"unit_id", req.UnitID,
		"ingot_index", req.IngotIndex)

	// Validate request
	if req.UnitID == "" {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, "unit_id is required")
		return
	}

	if req.IngotIndex < 0 || req.IngotIndex >= 1000 {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, "ingot_index must be 0-999")
		return
	}

	// Look up merkle result in cache
	merkleResult := h.proofCache.Get(req.UnitID)
	if merkleResult == nil {
		h.metrics.ProofRequestsFailure.Inc()
		h.respondJSON(w, http.StatusNotFound, map[string]interface{}{
			"unit_id": req.UnitID,
			"error":   "unit not found in proof cache",
		})
		return
	}

	// Generate merkle proof
	proof, err := merkleResult.GetProof(req.IngotIndex)
	if err != nil {
		h.metrics.ProofRequestsFailure.Inc()
		h.logger.Error("failed to generate proof",
			"unit_id", req.UnitID,
			"ingot_index", req.IngotIndex,
			"error", err)
		h.respondError(w, http.StatusInternalServerError, fmt.Sprintf("proof generation failed: %v", err))
		return
	}

	// Verify proof (sanity check)
	leafHash := merkleResult.TreeNodes[0][req.IngotIndex]
	verified := VerifyProof(leafHash, proof, merkleResult.MerkleRoot, req.IngotIndex)

	h.metrics.ProofRequestsSuccess.Inc()
	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"unit_id":      req.UnitID,
		"ingot_index":  req.IngotIndex,
		"merkle_root":  merkleResult.MerkleRoot,
		"tree_height":  merkleResult.TreeHeight,
		"proof":        proof,
		"proof_length": len(proof),
		"verified":     verified,
	})
}

// handleSignatureRequest handles GET /verify/signature/:unit_id
//
// Request: GET /verify/signature/RT-20251117-001
//
// Response (200):
//
//	{
//	  "unit_id": "RT-20251117-001",
//	  "signature": "abc123...",
//	  "public_key": "def456...",
//	  "merkle_root": "aabbccdd...",
//	  "minted_at": "2025-11-17T12:00:00Z",
//	  "signed_at": "2025-11-17T12:00:01Z"
//	}
//
// Response (404):
//
//	{
//	  "error": "signature not found for unit_id=RT-nonexistent"
//	}
func (h *VerificationHandler) handleSignatureRequest(w http.ResponseWriter, r *http.Request) {
	// Extract unit ID from URL path
	path := strings.TrimPrefix(r.URL.Path, "/verify/signature/")
	unitID := strings.TrimSpace(path)

	if unitID == "" {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, "unit_id is required")
		return
	}

	h.logger.Debug("signature lookup request", "unit_id", unitID)

	// Look up signature in archive
	record, err := h.signatureArchive.Get(unitID)
	if err != nil {
		h.respondJSON(w, http.StatusNotFound, map[string]interface{}{
			"error":   err.Error(),
			"unit_id": unitID,
		})
		return
	}

	// Return signature record
	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"unit_id":     record.UnitID,
		"signature":   record.Signature,
		"public_key":  record.PublicKey,
		"merkle_root": record.MerkleRoot,
		"minted_at":   record.MintedAt,
		"signed_at":   record.SignedAt,
	})
}

// handleCertificateVerification handles POST /verify/certificate
//
// Verifies if a Phase3RoboTorqUnit certificate is legitimate.
// Checks merkle_root existence in ProofCache and validates signature.
//
// Request body:
//
//	{
//	  "merkle_root": "abc123...",
//	  "unit_id": "RT-20251117-001" (optional)
//	}
//
// Response (200 - valid):
//
//	{
//	  "valid": true,
//	  "merkle_root": "abc123...",
//	  "unit_id": "RT-20251117-001",
//	  "minted_at": "2025-11-17T12:00:00Z",
//	  "tree_height": 10,
//	  "ingot_count": 1000
//	}
//
// Response (404 - invalid):
//
//	{
//	  "valid": false,
//	  "merkle_root": "abc123...",
//	  "error": "certificate not found in Mint records"
//	}
func (h *VerificationHandler) handleCertificateVerification(w http.ResponseWriter, r *http.Request) {
	timer := prometheus.NewTimer(h.metrics.ResponseLatency)
	defer timer.ObserveDuration()

	if r.Method != http.MethodPost {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusMethodNotAllowed, "only POST method allowed")
		return
	}

	// Parse request body
	var req struct {
		MerkleRoot string `json:"merkle_root"`
		UnitID     string `json:"unit_id"` // Optional
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, fmt.Sprintf("invalid JSON: %v", err))
		return
	}

	h.logger.Debug("certificate verification request",
		"merkle_root", req.MerkleRoot,
		"unit_id", req.UnitID)

	// Validate request
	if req.MerkleRoot == "" {
		h.metrics.InvalidRequestsTotal.Inc()
		h.respondError(w, http.StatusBadRequest, "merkle_root is required")
		return
	}

	// Look up by merkle_root in proof cache
	// ProofCache stores unitID → merkleResult, so we need reverse lookup
	var merkleResult *Level2MerkleResult
	var unitID string

	if req.UnitID != "" {
		// If unit_id provided, verify it matches merkle_root
		merkleResult = h.proofCache.Get(req.UnitID)
		if merkleResult == nil || merkleResult.MerkleRoot != req.MerkleRoot {
			h.respondJSON(w, http.StatusOK, map[string]interface{}{
				"valid":       false,
				"merkle_root": req.MerkleRoot,
				"unit_id":     req.UnitID,
				"error":       "merkle_root does not match unit_id or unit not found",
			})
			return
		}
		unitID = req.UnitID
	} else {
		// Search all cached units for matching merkle_root
		unitID = h.proofCache.FindByMerkleRoot(req.MerkleRoot)
		if unitID == "" {
			h.respondJSON(w, http.StatusOK, map[string]interface{}{
				"valid":       false,
				"merkle_root": req.MerkleRoot,
				"error":       "certificate not found in Mint records",
			})
			return
		}
		merkleResult = h.proofCache.Get(unitID)
	}

	// Get signature record (optional - for minted_at timestamp)
	var mintedAt string
	if signatureRecord, err := h.signatureArchive.Get(unitID); err == nil {
		mintedAt = signatureRecord.MintedAt
	}

	// Certificate is valid
	h.logger.Info("certificate verified",
		"unit_id", unitID,
		"merkle_root", req.MerkleRoot)

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"valid":        true,
		"merkle_root":  req.MerkleRoot,
		"unit_id":      unitID,
		"minted_at":    mintedAt,
		"tree_height":  merkleResult.TreeHeight,
		"ingot_count":  len(merkleResult.HashEntries),
		"verified_by":  "RoboTorq Mint",
		"verified_at":  prometheus.NewTimer(nil).ObserveDuration().String(),
	})
}

// handlePublicKey handles GET /public-key
//
// Returns the Mint's SPHINCS+ public key for signature verification.
// External parties (wallets, DistoDam, auditors) need this to verify
// signatures on Phase3RoboTorqUnits without access to the private key.
//
// Response (200):
//
//	{
//	  "algorithm": "SPHINCS+-SHA2-128f-simple",
//	  "public_key": "abc123def456...",
//	  "key_size_bytes": 32,
//	  "purpose": "Verify SPHINCS+ signatures on Phase3RoboTorqUnits"
//	}
//
// Usage:
//
//	curl http://mint:8081/public-key
func (h *VerificationHandler) handlePublicKey(w http.ResponseWriter, r *http.Request) {
	h.logger.Debug("public key request")

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"algorithm":      "SPHINCS+-SHA2-128f-simple",
		"public_key":     h.publicKey,
		"key_size_bytes": len(h.publicKey) / 2, // Hex string is 2x byte length
		"purpose":        "Verify SPHINCS+ signatures on Phase3RoboTorqUnits",
	})
}

// handleHealth handles GET /health
//
// Response (200):
//
//	{
//	  "status": "healthy",
//	  "cache_size": 42
//	}
func (h *VerificationHandler) handleHealth(w http.ResponseWriter, r *http.Request) {
	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"status":          "healthy",
		"cache_size":      h.proofCache.Size(),
		"signature_count": h.signatureArchive.Size(),
	})
}

// respondJSON writes a JSON response
func (h *VerificationHandler) respondJSON(w http.ResponseWriter, statusCode int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*") // CORS
	w.WriteHeader(statusCode)

	if err := json.NewEncoder(w).Encode(data); err != nil {
		h.logger.Error("failed to encode JSON response", "error", err)
	}
}

// respondError writes a JSON error response
func (h *VerificationHandler) respondError(w http.ResponseWriter, statusCode int, message string) {
	h.respondJSON(w, statusCode, map[string]interface{}{
		"error": message,
	})
}
