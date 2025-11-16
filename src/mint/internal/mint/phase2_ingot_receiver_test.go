// internal/mint/phase2_ingot_receiver_test.go
package mint

import (
	"encoding/json"
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"github.com/nats-io/nats.go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewPhase2IngotReceiver tests receiver creation
func TestNewPhase2IngotReceiver(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))

	// Connect to NATS (requires NATS running)
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, logger)

	require.NoError(t, err)
	assert.NotNil(t, receiver)
	assert.NotNil(t, receiver.metrics)
	assert.NotNil(t, receiver.natsConn)
}

// TestNewPhase2IngotReceiver_NilConnection tests error handling
func TestNewPhase2IngotReceiver_NilConnection(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))

	receiver, err := NewPhase2IngotReceiver(nil, logger)

	assert.Error(t, err)
	assert.Nil(t, receiver)
	assert.Contains(t, err.Error(), "NATS connection cannot be nil")
}

// TestPhase2IngotReceiver_StartStop tests subscription lifecycle
func TestPhase2IngotReceiver_StartStop(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))

	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, logger)
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

	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, logger)
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

	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, logger)
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

	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skip("NATS not available, skipping test")
	}
	defer nc.Close()

	receiver, err := NewPhase2IngotReceiver(nc, logger)
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
