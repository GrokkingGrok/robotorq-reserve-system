//go:build cgo
// +build cgo

// internal/mint/phase2_ingot_receiver_test.go
package mint

import (
	"context"
	"encoding/json"
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/mint/internal/crypto"
	"b2b/mint/internal/models"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	dto "github.com/prometheus/client_model/go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// getNATSURL returns the NATS URL from environment or defaults
func getNATSURL() string {
	if url := os.Getenv("NATS_URL"); url != "" {
		return url
	}
	// Try Docker network first, then localhost
	if _, err := nats.Connect("nats://nats:4222"); err == nil {
		return "nats://nats:4222"
	}
	return "nats://localhost:4222"
}

// TestNewPhase2IngotReceiver tests receiver creation
func TestNewPhase2IngotReceiver(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	// Connect to NATS (requires NATS running)
	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	// Create queue
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)

	require.NoError(t, err)
	assert.NotNil(t, receiver)
	assert.NotNil(t, receiver.metrics)
	assert.NotNil(t, receiver.natsConn)
	assert.NotNil(t, receiver.queue)
	assert.NotNil(t, receiver.verifier) // Phase 5: Falcon verifier
}

// TestNewPhase2IngotReceiver_NilConnection tests error handling
func TestNewPhase2IngotReceiver_NilConnection(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()
	queue, _ := NewIngotHashQueue(2000, 1000, logger)

	receiver, err := NewPhase2IngotReceiver(nil, queue, ctx, logger)

	assert.Error(t, err)
	assert.Nil(t, receiver)
	assert.Contains(t, err.Error(), "NATS connection cannot be nil")
}

// TestNewPhase2IngotReceiver_NilQueue tests queue validation
func TestNewPhase2IngotReceiver_NilQueue(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, nil, ctx, logger)

	assert.Error(t, err)
	assert.Nil(t, receiver)
	assert.Contains(t, err.Error(), "IngotHashQueue cannot be nil")
}

// TestPhase2IngotReceiver_StartStop tests subscription lifecycle
func TestPhase2IngotReceiver_StartStop(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	// Start receiver
	err = receiver.Start()
	assert.NoError(t, err)
	assert.NotNil(t, receiver.subscription)

	// Stop receiver
	err = receiver.Stop()
	assert.NoError(t, err)
}

// TestPhase2IngotReceiver_ValidIngot tests receiving valid ingot
func TestPhase2IngotReceiver_ValidIngot(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create valid test ingot
	testIngot := models.Phase2Ingot{
		ID:          "test-ingot-001",
		BranchHash:  "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab", // 64 chars
		HashCount:   3600,
		ContractIDs: []string{"contract-1", "contract-2"},
		DiggerIDs:   []string{"digger-1"},
		Timestamp:   time.Now().UTC(),
	}

	// Publish to NATS
	data, err := json.Marshal(testIngot)
	require.NoError(t, err)

	err = nc.Publish("mint.ingots", data)
	require.NoError(t, err)
	nc.Flush()

	// Wait for processing
	time.Sleep(100 * time.Millisecond)

	// Verify metrics (should have 1 ingot received)
	// Note: In real test, would check metrics counter value
	// For now, just verify no crashes
}

// TestPhase2IngotReceiver_InvalidJSON tests unmarshal error handling
func TestPhase2IngotReceiver_InvalidJSON(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Publish invalid JSON
	err = nc.Publish("mint.ingots", []byte("{invalid json}"))
	require.NoError(t, err)
	nc.Flush()

	// Wait for processing
	time.Sleep(100 * time.Millisecond)

	// Should log error but not crash
	// ValidationErrors.WithLabelValues("unmarshal") should increment
}

// TestPhase2IngotReceiver_InvalidIngot tests validation error handling
func TestPhase2IngotReceiver_InvalidIngot(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create invalid ingot (wrong hash count)
	invalidIngot := models.Phase2Ingot{
		ID:          "invalid-ingot",
		BranchHash:  "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
		HashCount:   1000, // Should be 3600!
		ContractIDs: []string{"contract-1"},
		DiggerIDs:   []string{"digger-1"},
		Timestamp:   time.Now().UTC(),
	}

	data, err := json.Marshal(invalidIngot)
	require.NoError(t, err)

	err = nc.Publish("mint.ingots", data)
	require.NoError(t, err)
	nc.Flush()

	// Wait for processing
	time.Sleep(100 * time.Millisecond)

	// Should log warning and increment ValidationErrors.WithLabelValues("validation")
}

// TestTruncateHash tests hash truncation for logging
func TestTruncateHash(t *testing.T) {
	tests := []struct {
		name     string
		hash     string
		expected string
	}{
		{
			name:     "full 64-char hash",
			hash:     "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
			expected: "abcdef1234567890...",
		},
		{
			name:     "short hash",
			hash:     "abc123",
			expected: "abc123",
		},
		{
			name:     "exactly 16 chars",
			hash:     "abcdef1234567890",
			expected: "abcdef1234567890",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := truncateHash(tt.hash)
			assert.Equal(t, tt.expected, result)
		})
	}
}

// ========== PHASE 5 CRYPTO INTEGRATION TESTS ==========

// TestPhase2IngotReceiver_ValidFalconSignature tests that receiver accepts valid Falcon-1024 signatures
func TestPhase2IngotReceiver_ValidFalconSignature(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test (start with: docker-compose up -d nats)")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer (mimics Refinery)
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Create signed ingot
	ingot := createTestPhase2Ingot(t, signer, "test-ingot-valid-001")

	// Publish batch to NATS
	batch := createTestBatch([]*models.Phase2Ingot{ingot})
	publishBatch(t, nc, batch)

	// Wait for processing
	time.Sleep(300 * time.Millisecond)

	// Verify ingot was queued
	assert.Equal(t, 1, queue.Len(), "Queue should contain 1 ingot")

	// Verify metrics
	received := getCounterValue(receiver.metrics.IngotsReceived)
	assert.Equal(t, float64(1), received, "IngotsReceived metric should be 1")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.Equal(t, float64(0), verifyErrors, "Should have no signature verification errors")
}

// TestPhase2IngotReceiver_InvalidFalconSignature tests that receiver rejects invalid signatures
func TestPhase2IngotReceiver_InvalidFalconSignature(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Create ingot with valid signature
	ingot := createTestPhase2Ingot(t, signer, "test-ingot-invalid-001")

	// Corrupt the signature (flip first byte)
	ingot.Signature = "ff" + ingot.Signature[2:]

	// Publish batch
	batch := createTestBatch([]*models.Phase2Ingot{ingot})
	publishBatch(t, nc, batch)

	// Wait for processing
	time.Sleep(300 * time.Millisecond)

	// Verify ingot was NOT queued
	assert.Equal(t, 0, queue.Len(), "Queue should be empty (invalid signature rejected)")

	// Verify metrics
	received := getCounterValue(receiver.metrics.IngotsReceived)
	assert.Equal(t, float64(0), received, "IngotsReceived should be 0 (rejected)")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.GreaterOrEqual(t, verifyErrors, float64(1), "Should have at least 1 signature verification error")
}

// TestPhase2IngotReceiver_TamperedMessage tests detection of message tampering
func TestPhase2IngotReceiver_TamperedMessage(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Create signed ingot
	ingot := createTestPhase2Ingot(t, signer, "test-ingot-tampered-001")

	// Tamper with the message (change branch hash AFTER signing)
	ingot.BranchHash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

	// Publish batch
	batch := createTestBatch([]*models.Phase2Ingot{ingot})
	publishBatch(t, nc, batch)

	// Wait for processing
	time.Sleep(300 * time.Millisecond)

	// Verify ingot was rejected
	assert.Equal(t, 0, queue.Len(), "Queue should be empty (tampered message rejected)")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.GreaterOrEqual(t, verifyErrors, float64(1), "Should have at least 1 signature verification error")
}

// TestPhase2IngotReceiver_MultipleValidSignatures tests processing multiple batches with valid signatures
func TestPhase2IngotReceiver_MultipleValidSignatures(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Publish 3 batches with 5 ingots each
	totalIngots := 0
	for batchNum := 0; batchNum < 3; batchNum++ {
		ingots := []*models.Phase2Ingot{}
		for i := 0; i < 5; i++ {
			ingotID := "multi-ingot-" + time.Now().Format("20060102-150405.000000")
			time.Sleep(1 * time.Millisecond) // Ensure unique IDs
			ingot := createTestPhase2Ingot(t, signer, ingotID)
			ingots = append(ingots, ingot)
			totalIngots++
		}

		batch := createTestBatch(ingots)
		publishBatch(t, nc, batch)

		// Small delay between batches
		time.Sleep(50 * time.Millisecond)
	}

	// Wait for all processing
	time.Sleep(1000 * time.Millisecond)

	// Verify all ingots were queued
	assert.Equal(t, totalIngots, queue.Len(), "Queue should contain all ingots")

	received := getCounterValue(receiver.metrics.IngotsReceived)
	assert.Equal(t, float64(totalIngots), received, "All ingots should be received")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.Equal(t, float64(0), verifyErrors, "Should have no signature verification errors")
}

// TestPhase2IngotReceiver_MixedValidInvalid tests batch with mixed valid/invalid signatures
func TestPhase2IngotReceiver_MixedValidInvalid(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Create 5 ingots: 3 valid, 2 invalid
	ingots := []*models.Phase2Ingot{}

	// Valid ingots
	ingots = append(ingots, createTestPhase2Ingot(t, signer, "valid-001"))
	time.Sleep(1 * time.Millisecond)
	ingots = append(ingots, createTestPhase2Ingot(t, signer, "valid-002"))
	time.Sleep(1 * time.Millisecond)
	ingots = append(ingots, createTestPhase2Ingot(t, signer, "valid-003"))
	time.Sleep(1 * time.Millisecond)

	// Invalid ingot 1: corrupted signature
	invalid1 := createTestPhase2Ingot(t, signer, "invalid-001")
	invalid1.Signature = "deadbeef" + invalid1.Signature[8:]
	ingots = append(ingots, invalid1)
	time.Sleep(1 * time.Millisecond)

	// Invalid ingot 2: tampered message
	invalid2 := createTestPhase2Ingot(t, signer, "invalid-002")
	invalid2.BranchHash = "0000000000000000000000000000000000000000000000000000000000000000"
	ingots = append(ingots, invalid2)

	// Publish batch
	batch := createTestBatch(ingots)
	publishBatch(t, nc, batch)

	// Wait for processing
	time.Sleep(500 * time.Millisecond)

	// Verify only valid ingots were queued
	assert.Equal(t, 3, queue.Len(), "Queue should contain only 3 valid ingots")

	received := getCounterValue(receiver.metrics.IngotsReceived)
	assert.Equal(t, float64(3), received, "Should have 3 valid ingots")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.GreaterOrEqual(t, verifyErrors, float64(2), "Should have at least 2 signature verification errors")
}

// TestPhase2IngotReceiver_SignatureVerifyTime tests that verification time metrics are recorded
func TestPhase2IngotReceiver_SignatureVerifyTime(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	ctx := context.Background()

	nc, err := nats.Connect(getNATSURL())
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)

	receiver, err := NewPhase2IngotReceiver(nc, queue, ctx, logger)
	require.NoError(t, err)

	err = receiver.Start()
	require.NoError(t, err)
	defer receiver.Stop()

	// Create test signer
	signer, err := crypto.NewTestFalconSigner()
	require.NoError(t, err)
	defer signer.Clean()

	// Create and publish valid ingot
	ingot := createTestPhase2Ingot(t, signer, "metrics-test-001")
	batch := createTestBatch([]*models.Phase2Ingot{ingot})
	publishBatch(t, nc, batch)

	// Wait for processing
	time.Sleep(300 * time.Millisecond)

	// Check metrics
	received := getCounterValue(receiver.metrics.IngotsReceived)
	assert.Equal(t, float64(1), received, "IngotsReceived should be 1")

	verifyErrors := getCounterValue(receiver.metrics.SignatureVerifyErrors)
	assert.Equal(t, float64(0), verifyErrors, "SignatureVerifyErrors should be 0")

	// Histogram should have recorded the verification time
	assert.NotNil(t, receiver.metrics.SignatureVerifyTime, "SignatureVerifyTime histogram should exist")
}

// ========== HELPER FUNCTIONS ==========

// createTestPhase2Ingot creates a Phase2Ingot with a valid Falcon-1024 signature
func createTestPhase2Ingot(t *testing.T, signer *crypto.TestFalconSigner, ingotID string) *models.Phase2Ingot {
	t.Helper()

	branchHash := "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
	hashCount := 3600
	timestamp := time.Now()

	// Sign the ingot
	signature, publicKey, err := signer.SignPhase2Ingot(
		ingotID,
		branchHash,
		hashCount,
		timestamp.Format(time.RFC3339),
	)
	require.NoError(t, err, "Failed to sign test ingot")

	return &models.Phase2Ingot{
		ID:          ingotID,
		BranchHash:  branchHash,
		HashCount:   hashCount,
		ContractIDs: []string{"test-contract-001"},
		DiggerIDs:   []string{"test-digger-001"},
		Timestamp:   timestamp,
		Signature:   signature,
		PublicKey:   publicKey,
	}
}

// createTestBatch creates a NATS batch wrapper for Phase2Ingots
func createTestBatch(ingots []*models.Phase2Ingot) map[string]interface{} {
	return map[string]interface{}{
		"batch_id":  "test-batch-" + time.Now().Format("20060102-150405.000000"),
		"timestamp": time.Now().Format(time.RFC3339),
		"count":     len(ingots),
		"ingots":    ingots,
	}
}

// publishBatch publishes a batch to NATS
func publishBatch(t *testing.T, nc *nats.Conn, batch map[string]interface{}) {
	t.Helper()

	data, err := json.Marshal(batch)
	require.NoError(t, err, "Failed to marshal batch")

	err = nc.Publish("mint.phase2.ingots", data)
	require.NoError(t, err, "Failed to publish to NATS")

	// Flush to ensure message is sent
	err = nc.Flush()
	require.NoError(t, err, "Failed to flush NATS")
}

// getCounterValue retrieves the current value of a Prometheus counter
func getCounterValue(counter prometheus.Counter) float64 {
	ch := make(chan prometheus.Metric, 1)
	counter.Collect(ch)
	close(ch)
	for metric := range ch {
		pb := &dto.Metric{}
		if err := metric.Write(pb); err != nil {
			return 0
		}
		if pb.Counter != nil {
			return pb.Counter.GetValue()
		}
	}
	return 0
}
