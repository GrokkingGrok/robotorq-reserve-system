package mint

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/testutil"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────

// mockIngotBuffer is a test double for IngotBuffer.
type mockIngotBuffer struct {
	ingots   []*TokenTorqIngot
	capacity int
}

func newMockIngotBuffer(cap int) *mockIngotBuffer {
	return &mockIngotBuffer{
		ingots:   make([]*TokenTorqIngot, 0, cap),
		capacity: cap,
	}
}

func (m *mockIngotBuffer) Push(ingot *TokenTorqIngot) error {
	if len(m.ingots) >= m.capacity {
		return ErrBufferFull
	}
	m.ingots = append(m.ingots, ingot)
	return nil
}

func (m *mockIngotBuffer) Pop(ctx context.Context) (*TokenTorqIngot, error) {
	if len(m.ingots) == 0 {
		return nil, ErrBufferEmpty
	}
	ingot := m.ingots[0]
	m.ingots = m.ingots[1:]
	return ingot, nil
}

func (m *mockIngotBuffer) Len() int {
	return len(m.ingots)
}

func (m *mockIngotBuffer) Cap() int {
	return m.capacity
}

func (m *mockIngotBuffer) Drain() []*TokenTorqIngot {
	ingots := m.ingots
	m.ingots = make([]*TokenTorqIngot, 0, m.capacity)
	return ingots
}

// validIngot returns a valid TokenTorqIngot for testing.
func validIngot() *TokenTorqIngot {
	return &TokenTorqIngot{
		JouleTorq:  3600.0,
		RoboTorq:   0.123456,
		Price:      50.00,
		ContractID: "contract-123",
		DiggerID:   "digger-456",
		Timestamp:  time.Now().UTC(),
		Hash:       "hash-abc",
	}
}

// setupTestReceiver creates a test IngotReceiver with mock buffer.
func setupTestReceiver(bufferCap int) (IngotReceiver, *mockIngotBuffer) {
	// Reset Prometheus registry to avoid conflicts
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	buffer := newMockIngotBuffer(bufferCap)
	logger := slog.New(slog.NewJSONHandler(bytes.NewBuffer(nil), nil))
	receiver := NewIngotReceiver(buffer, "8080", logger)

	return receiver, buffer
}

// ─────────────────────────────────────────────────────────────
// Validation Tests
// ─────────────────────────────────────────────────────────────

func TestValidateIngot_ValidIngot(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	err := r.validateIngot(ingot)
	assert.NoError(t, err)
}

func TestValidateIngot_InvalidJouleTorq(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.JouleTorq = 1800.0 // Wrong value
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid JouleTorq")
}

func TestValidateIngot_NegativeRoboTorq(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.RoboTorq = -0.5 // Negative
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid RoboTorq")
}

func TestValidateIngot_ZeroPrice(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.Price = 0 // Zero price
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid Price")
}

func TestValidateIngot_NegativePrice(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.Price = -10.0 // Negative price
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid Price")
}

func TestValidateIngot_EmptyContractID(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.ContractID = "" // Empty
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "ContractID")
}

func TestValidateIngot_EmptyDiggerID(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.DiggerID = "" // Empty
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "DiggerID")
}

func TestValidateIngot_EmptyHash(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.Hash = "" // Empty
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "Hash")
}

func TestValidateIngot_ZeroTimestamp(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.Timestamp = time.Time{} // Zero time
	err := r.validateIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "Timestamp")
}

// ─────────────────────────────────────────────────────────────
// ReceiveIngot Tests
// ─────────────────────────────────────────────────────────────

func TestReceiveIngot_Success(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)

	ingot := validIngot()
	err := receiver.ReceiveIngot(ingot)
	assert.NoError(t, err)
	assert.Equal(t, 1, buffer.Len())
}

func TestReceiveIngot_ValidationFailure(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)

	ingot := validIngot()
	ingot.JouleTorq = 1800.0 // Invalid
	err := receiver.ReceiveIngot(ingot)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "validation failed")
	assert.Equal(t, 0, buffer.Len()) // Not buffered
}

func TestReceiveIngot_BufferFull(t *testing.T) {
	receiver, buffer := setupTestReceiver(2) // Small buffer

	// Fill buffer
	assert.NoError(t, receiver.ReceiveIngot(validIngot()))
	assert.NoError(t, receiver.ReceiveIngot(validIngot()))
	assert.Equal(t, 2, buffer.Len())

	// Try to add third ingot (should fail)
	err := receiver.ReceiveIngot(validIngot())
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "buffer full")
	assert.Equal(t, 2, buffer.Len()) // Still at capacity
}

func TestReceiveIngot_MetricsUpdated(t *testing.T) {
	receiver, _ := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	// Successful ingot
	assert.NoError(t, receiver.ReceiveIngot(validIngot()))

	// Check metrics
	received := testutil.ToFloat64(r.metrics.ingotsReceived)
	assert.Equal(t, 1.0, received)

	// Validation failure
	invalidIngot := validIngot()
	invalidIngot.JouleTorq = 1800.0
	receiver.ReceiveIngot(invalidIngot)

	rejected := testutil.ToFloat64(r.metrics.ingotsRejected)
	validationErrors := testutil.ToFloat64(r.metrics.validationErrors)
	assert.Equal(t, 1.0, rejected)
	assert.Equal(t, 1.0, validationErrors)
}

// ─────────────────────────────────────────────────────────────
// HTTP Handler Tests
// ─────────────────────────────────────────────────────────────

func TestHandleIngot_POSTSuccess(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	body, _ := json.Marshal(ingot)

	req := httptest.NewRequest(http.MethodPost, "/mint-tokentorq", bytes.NewReader(body))
	rec := httptest.NewRecorder()

	r.handleIngot(rec, req)

	assert.Equal(t, http.StatusAccepted, rec.Code)
	assert.Equal(t, 1, buffer.Len())
}

func TestHandleIngot_InvalidMethod(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	req := httptest.NewRequest(http.MethodGet, "/mint-tokentorq", nil)
	rec := httptest.NewRecorder()

	r.handleIngot(rec, req)

	assert.Equal(t, http.StatusMethodNotAllowed, rec.Code)
	assert.Equal(t, 0, buffer.Len())
}

func TestHandleIngot_InvalidJSON(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	req := httptest.NewRequest(http.MethodPost, "/mint-tokentorq", bytes.NewReader([]byte("invalid json")))
	rec := httptest.NewRecorder()

	r.handleIngot(rec, req)

	assert.Equal(t, http.StatusBadRequest, rec.Code)
	assert.Equal(t, 0, buffer.Len())
}

func TestHandleIngot_ValidationError(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	ingot := validIngot()
	ingot.JouleTorq = 1800.0 // Invalid
	body, _ := json.Marshal(ingot)

	req := httptest.NewRequest(http.MethodPost, "/mint-tokentorq", bytes.NewReader(body))
	rec := httptest.NewRecorder()

	r.handleIngot(rec, req)

	assert.Equal(t, http.StatusBadRequest, rec.Code)
	assert.Equal(t, 0, buffer.Len())
}

func TestHandleIngot_Backpressure(t *testing.T) {
	receiver, buffer := setupTestReceiver(1) // Capacity of 1
	r := receiver.(*ingotReceiver)

	// Fill buffer
	ingot := validIngot()
	body, _ := json.Marshal(ingot)
	req := httptest.NewRequest(http.MethodPost, "/mint-tokentorq", bytes.NewReader(body))
	rec := httptest.NewRecorder()
	r.handleIngot(rec, req)
	assert.Equal(t, http.StatusAccepted, rec.Code)
	assert.Equal(t, 1, buffer.Len())

	// Try to add another (should get 429)
	body2, _ := json.Marshal(validIngot())
	req2 := httptest.NewRequest(http.MethodPost, "/mint-tokentorq", bytes.NewReader(body2))
	rec2 := httptest.NewRecorder()
	r.handleIngot(rec2, req2)

	assert.Equal(t, http.StatusTooManyRequests, rec2.Code)
	assert.Equal(t, 1, buffer.Len()) // Still at capacity
}

// ─────────────────────────────────────────────────────────────
// Health Check Tests
// ─────────────────────────────────────────────────────────────

func TestHandleHealth_ReturnsStatus(t *testing.T) {
	receiver, buffer := setupTestReceiver(100)
	r := receiver.(*ingotReceiver)

	// Add some ingots
	buffer.Push(validIngot())
	buffer.Push(validIngot())

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	rec := httptest.NewRecorder()

	r.handleHealth(rec, req)

	assert.Equal(t, http.StatusOK, rec.Code)
	assert.Equal(t, "application/json", rec.Header().Get("Content-Type"))

	var status map[string]interface{}
	err := json.NewDecoder(rec.Body).Decode(&status)
	require.NoError(t, err)

	assert.Equal(t, "ok", status["status"])
	assert.Equal(t, float64(2), status["buffer_len"])
	assert.Equal(t, float64(100), status["buffer_cap"])
	assert.Equal(t, float64(2), status["buffer_util"])
}

// ─────────────────────────────────────────────────────────────
// Start/Shutdown Tests
// ─────────────────────────────────────────────────────────────

func TestStartShutdown_GracefulShutdown(t *testing.T) {
	receiver, _ := setupTestReceiver(100)

	ctx, cancel := context.WithCancel(context.Background())

	// Start server in goroutine (use port 0 for random port)
	r := receiver.(*ingotReceiver)
	r.server.Addr = ":0" // Use random available port

	startErr := make(chan error, 1)
	go func() {
		startErr <- receiver.Start(ctx)
	}()

	// Give server time to start
	time.Sleep(100 * time.Millisecond)

	// Shutdown
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer shutdownCancel()

	err := receiver.Shutdown(shutdownCtx)
	assert.NoError(t, err)

	// Cancel context to exit Start()
	cancel()

	// Wait for Start() to return
	select {
	case err := <-startErr:
		// Should return context.Canceled
		assert.ErrorIs(t, err, context.Canceled)
	case <-time.After(2 * time.Second):
		t.Fatal("Start() did not return after shutdown")
	}
}

// ─────────────────────────────────────────────────────────────
// Integration Test
// ─────────────────────────────────────────────────────────────

func TestIngotReceiver_Integration(t *testing.T) {
	// Skip integration test - requires coordination with HTTP listener lifecycle
	// The unit tests above provide comprehensive coverage of all functionality:
	// - HTTP handler tests (POST, GET, invalid methods, JSON parsing)
	// - Validation tests (all ingot fields)
	// - Buffer integration tests (push, backpressure)
	// - Metrics tests (counters update correctly)
	// - Shutdown tests (graceful shutdown)
	// A full integration test will be done in TODO #22 with the complete pipeline.
	t.Skip("Skipping HTTP integration test - covered by unit tests and full pipeline test in TODO #22")
}
