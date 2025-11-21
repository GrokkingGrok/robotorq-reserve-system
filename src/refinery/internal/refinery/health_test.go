package refinery

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

// Mock implementations for testing
type mockHealthChecker struct {
	queueSize int
	capacity  int
}

func (m *mockHealthChecker) GetQueueSize() int {
	return m.queueSize
}

func (m *mockHealthChecker) GetCapacity() int {
	return m.capacity
}

type mockMintHealthChecker struct {
	connected bool
	status    string
}

func (m *mockMintHealthChecker) IsConnected() bool {
	return m.connected
}

func (m *mockMintHealthChecker) GetStatus() string {
	return m.status
}

type mockAssemblerHealthChecker struct {
	accumulatedUnits int
	completedIngots  int
}

func (m *mockAssemblerHealthChecker) GetAccumulatedUnits() int {
	return m.accumulatedUnits
}

func (m *mockAssemblerHealthChecker) GetCompletedIngotsCount() int {
	return m.completedIngots
}

func TestNewHealthHandler(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

	hh := NewHealthHandler(qm, mc, ac)

	if hh.queueMgr == nil {
		t.Errorf("queueMgr should not be nil")
	}
	if hh.mintClient == nil {
		t.Errorf("mintClient should not be nil")
	}
	if hh.assembler == nil {
		t.Errorf("assembler should not be nil")
	}
}

func TestHealthHandler_Healthy(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 500, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 1000, completedIngots: 5}

	hh := NewHealthHandler(qm, mc, ac)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("Expected status code %d, got %d", http.StatusOK, w.Code)
	}

	var health HealthResponse
	err := json.Unmarshal(w.Body.Bytes(), &health)
	if err != nil {
		t.Fatalf("Failed to unmarshal response: %v", err)
	}

	if health.Status != "healthy" {
		t.Errorf("Expected status 'healthy', got '%s'", health.Status)
	}

	if health.Queue.HashQueueSize != 500 {
		t.Errorf("Expected queue size 500, got %d", health.Queue.HashQueueSize)
	}

	if health.Queue.HashQueueCapacity != 1000 {
		t.Errorf("Expected queue capacity 1000, got %d", health.Queue.HashQueueCapacity)
	}

	expectedUsage := 50.0
	if health.Queue.HashQueueUsage != expectedUsage {
		t.Errorf("Expected queue usage %.1f%%, got %.1f%%", expectedUsage, health.Queue.HashQueueUsage)
	}

	if !health.NATS.Connected {
		t.Errorf("Expected NATS connected, got %v", health.NATS.Connected)
	}

	if health.IngotAssembly.CompletedIngots != 5 {
		t.Errorf("Expected 5 completed ingots, got %d", health.IngotAssembly.CompletedIngots)
	}
}

func TestHealthHandler_Degraded_NATSDisconnected(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 200, capacity: 1000}
	mc := &mockMintHealthChecker{connected: false, status: "DISCONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 500, completedIngots: 2}

	hh := NewHealthHandler(qm, mc, ac)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)

	if w.Code != http.StatusServiceUnavailable {
		t.Errorf("Expected status code %d, got %d", http.StatusServiceUnavailable, w.Code)
	}

	var health HealthResponse
	json.Unmarshal(w.Body.Bytes(), &health)

	if health.Status != "degraded" {
		t.Errorf("Expected status 'degraded', got '%s'", health.Status)
	}

	if health.NATS.Connected {
		t.Errorf("Expected NATS disconnected")
	}
}

func TestHealthHandler_Warning_QueueNearCapacity(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 950, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

	hh := NewHealthHandler(qm, mc, ac)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("Expected status code %d (still operational), got %d", http.StatusOK, w.Code)
	}

	var health HealthResponse
	json.Unmarshal(w.Body.Bytes(), &health)

	if health.Status != "warning" {
		t.Errorf("Expected status 'warning', got '%s'", health.Status)
	}

	if health.Queue.HashQueueUsage != 95.0 {
		t.Errorf("Expected queue usage 95%%, got %.1f%%", health.Queue.HashQueueUsage)
	}
}

func TestHealthHandler_MethodNotAllowed(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

	hh := NewHealthHandler(qm, mc, ac)

	tests := []struct {
		method string
	}{
		{"POST"},
		{"PUT"},
		{"DELETE"},
		{"PATCH"},
	}

	for _, tt := range tests {
		t.Run(tt.method, func(t *testing.T) {
			req := httptest.NewRequest(tt.method, "/health", nil)
			w := httptest.NewRecorder()

			hh.HTTPHandler(w, req)

			if w.Code != http.StatusMethodNotAllowed {
				t.Errorf("Expected status code %d, got %d", http.StatusMethodNotAllowed, w.Code)
			}
		})
	}
}

func TestHealthHandler_ProgressCalculation(t *testing.T) {
	tests := []struct {
		name             string
		accumulatedH     int
		expectedProgress float64
	}{
		{
			name:             "empty",
			accumulatedH:     0,
			expectedProgress: 0.0,
		},
		{
			name:             "1800_hashes_50_percent",
			accumulatedH:     1800,
			expectedProgress: 50.0,
		},
		{
			name:             "3600_hashes_100_percent",
			accumulatedH:     3600,
			expectedProgress: 100.0,
		},
		{
			name:             "over_3600_capped_at_100",
			accumulatedH:     5000,
			expectedProgress: 100.0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
			mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
			ac := &mockAssemblerHealthChecker{accumulatedUnits: tt.accumulatedH, completedIngots: 0}

			hh := NewHealthHandler(qm, mc, ac)

			req := httptest.NewRequest("GET", "/health", nil)
			w := httptest.NewRecorder()

			hh.HTTPHandler(w, req)

			var health HealthResponse
			json.Unmarshal(w.Body.Bytes(), &health)

			if health.IngotAssembly.ProgressPercent != tt.expectedProgress {
				t.Errorf("Expected progress %.1f%%, got %.1f%%", tt.expectedProgress, health.IngotAssembly.ProgressPercent)
			}
		})
	}
}

func TestHealthHandler_QueueUsageCalculation(t *testing.T) {
	tests := []struct {
		name          string
		queueSize     int
		capacity      int
		expectedUsage float64
	}{
		{
			name:          "empty_queue",
			queueSize:     0,
			capacity:      1000,
			expectedUsage: 0.0,
		},
		{
			name:          "half_full",
			queueSize:     500,
			capacity:      1000,
			expectedUsage: 50.0,
		},
		{
			name:          "full",
			queueSize:     1000,
			capacity:      1000,
			expectedUsage: 100.0,
		},
		{
			name:          "zero_capacity",
			queueSize:     0,
			capacity:      0,
			expectedUsage: 0.0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			qm := &mockHealthChecker{queueSize: tt.queueSize, capacity: tt.capacity}
			mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
			ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

			hh := NewHealthHandler(qm, mc, ac)

			req := httptest.NewRequest("GET", "/health", nil)
			w := httptest.NewRecorder()

			hh.HTTPHandler(w, req)

			var health HealthResponse
			json.Unmarshal(w.Body.Bytes(), &health)

			if health.Queue.HashQueueUsage != tt.expectedUsage {
				t.Errorf("Expected usage %.1f%%, got %.1f%%", tt.expectedUsage, health.Queue.HashQueueUsage)
			}
		})
	}
}

func TestHealthHandler_ResponseStructure(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 500, completedIngots: 3}

	hh := NewHealthHandler(qm, mc, ac)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)

	var health HealthResponse
	err := json.Unmarshal(w.Body.Bytes(), &health)
	if err != nil {
		t.Fatalf("Failed to unmarshal response: %v", err)
	}

	// Verify all fields are populated
	if health.Status == "" {
		t.Errorf("Status should not be empty")
	}

	if health.Timestamp.IsZero() {
		t.Errorf("Timestamp should not be zero")
	}

	if health.Queue.HashQueueSize < 0 {
		t.Errorf("Queue size should be non-negative")
	}

	if health.NATS.Status == "" {
		t.Errorf("NATS status should not be empty")
	}

	if health.IngotAssembly.CompletedIngots < 0 {
		t.Errorf("Completed ingots should be non-negative")
	}
}

func TestHealthHandler_ContentType(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

	hh := NewHealthHandler(qm, mc, ac)

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)

	contentType := w.Header().Get("Content-Type")
	if contentType != "application/json" {
		t.Errorf("Expected Content-Type 'application/json', got '%s'", contentType)
	}
}

func TestHealthHandler_TimestampAccuracy(t *testing.T) {
	qm := &mockHealthChecker{queueSize: 100, capacity: 1000}
	mc := &mockMintHealthChecker{connected: true, status: "CONNECTED"}
	ac := &mockAssemblerHealthChecker{accumulatedUnits: 0, completedIngots: 0}

	hh := NewHealthHandler(qm, mc, ac)

	before := time.Now().UTC()
	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hh.HTTPHandler(w, req)
	after := time.Now().UTC()

	var health HealthResponse
	json.Unmarshal(w.Body.Bytes(), &health)

	if health.Timestamp.Before(before) || health.Timestamp.After(after) {
		t.Errorf("Timestamp should be between request and response time")
	}
}
