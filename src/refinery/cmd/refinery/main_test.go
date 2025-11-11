package main

import (
	"context"
	"encoding/json"
	"net/http"
	"testing"
	"time"
)

// ─────────────────────────────────────────────────────────────
// Helper: Wait for ingots to appear
// ─────────────────────────────────────────────────────────────
func waitForIngots(r *Refinery, count int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if int(r.ingotCnt.Load()) >= count {
			return true
		}
		time.Sleep(10 * time.Millisecond)
	}
	return false
}

// ─────────────────────────────────────────────────────────────
// Helper: Create a test refinery
// ─────────────────────────────────────────────────────────────
func newTestRefinery(t *testing.T) *Refinery {
	r := NewRefinery()
	t.Cleanup(func() {
		r.cancel()
		r.wg.Wait()
	})
	return r
}

// mockResponseWriter is a minimal http.ResponseWriter used in tests
// it captures the written body and status code for assertions.
type mockResponseWriter struct {
	header     http.Header
	statusCode int
	body       []byte
}

func (m *mockResponseWriter) Header() http.Header {
	if m.header == nil {
		m.header = make(http.Header)
	}
	return m.header
}

func (m *mockResponseWriter) Write(b []byte) (int, error) {
	// default to 200 if WriteHeader wasn't called explicitly
	if m.statusCode == 0 {
		m.statusCode = http.StatusOK
	}
	m.body = append(m.body, b...)
	return len(b), nil
}

func (m *mockResponseWriter) WriteHeader(statusCode int) {
	m.statusCode = statusCode
}

// ─────────────────────────────────────────────────────────────
// Test: Basic ingot minting
// ─────────────────────────────────────────────────────────────
func TestRefinery_BasicMint(t *testing.T) {
	r := newTestRefinery(t)

	go func() {
		// Add 2 joules and 2 robos (enough for 2 ingots)
		r.addJoule(JouleTorq{Amount: JoulePerIngot})
		r.addJoule(JouleTorq{Amount: JoulePerIngot})
		r.addRobo(RoboTorq{Amount: 1, Price: 100})
		r.addRobo(RoboTorq{Amount: 1, Price: 200})
	}()

	if !waitForIngots(r, 2, 2*time.Second) {
		t.Fatalf("expected 2 ingots, got %d", r.ingotCnt.Load())
	}
}

// ─────────────────────────────────────────────────────────────
// Test: Queue fills but does not block
// ─────────────────────────────────────────────────────────────
func TestRefinery_QueueFull(t *testing.T) {
	r := newTestRefinery(t)

	for i := 0; i < ChannelBuffer; i++ {
		if err := r.addJoule(JouleTorq{Amount: 1}); err != nil {
			t.Fatalf("unexpected error adding joule: %v", err)
		}
	}

	// Next add should fail (buffer full)
	if err := r.addJoule(JouleTorq{Amount: 1}); err == nil {
		t.Fatalf("expected error adding joule to full buffer, got nil")
	}
}

// ─────────────────────────────────────────────────────────────
// Test: Robo price ordering
// ─────────────────────────────────────────────────────────────
func TestRefinery_RoboPriceLimit(t *testing.T) {
	r := newTestRefinery(t)

	r.addJoule(JouleTorq{Amount: JoulePerIngot})
	r.addJoule(JouleTorq{Amount: JoulePerIngot})

	r.addRobo(RoboTorq{Amount: 1, Price: 50})
	r.addRobo(RoboTorq{Amount: 1, Price: 200})

	if !waitForIngots(r, 2, 2*time.Second) {
		t.Fatalf("expected 2 ingots, got %d", r.ingotCnt.Load())
	}
}

// ─────────────────────────────────────────────────────────────
// Test: Concurrent ingot creation
// ─────────────────────────────────────────────────────────────
func TestRefinery_ConcurrentMinting(t *testing.T) {
	r := newTestRefinery(t)

	_, cancel := context.WithCancel(context.Background())
	defer cancel()

	for i := 0; i < 50; i++ {
		go func() {
			r.addJoule(JouleTorq{Amount: JoulePerIngot})
			r.addRobo(RoboTorq{Amount: 1, Price: 100})
		}()
	}

	if !waitForIngots(r, 50, 5*time.Second) {
		t.Fatalf("expected 50 ingots, got %d", r.ingotCnt.Load())
	}
}

// ─────────────────────────────────────────────────────────────
// Test: Health endpoint JSON
// ─────────────────────────────────────────────────────────────
func TestRefinery_HealthHandler(t *testing.T) {
	r := newTestRefinery(t)

	// Create a mock response writer
	w := newMockResponseWriter()

	// First call generates cached health
	r.healthHandler(w, nil)

	if w.statusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.statusCode)
	}
	if len(w.body) == 0 {
		t.Fatal("expected non-empty health JSON")
	}

	// Optional: Validate JSON
	var health struct {
		Status string `json:"status"`
	}
	if err := json.Unmarshal(w.body, &health); err != nil {
		t.Fatal("health response is not valid JSON")
	}
	if health.Status != "ok" {
		t.Fatalf("expected status 'ok', got '%s'", health.Status)
	}
}

func newMockResponseWriter() *mockResponseWriter {
	return &mockResponseWriter{
		header: make(http.Header),
	}
}

// ─────────────────────────────────────────────────────────────
