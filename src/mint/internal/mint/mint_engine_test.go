package mint

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"sync"
	"testing"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Mock Implementations
// ─────────────────────────────────────────────────────────────

// mockBatchHasher is a test double for BatchHasher.
type mockBatchHasher struct {
	shouldFail bool
	hashCalls  int
	mu         sync.Mutex
}

func (m *mockBatchHasher) Hash(batch []*TokenTorqIngot) (string, float64, float64, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.hashCalls++

	if m.shouldFail {
		return "", 0, 0, fmt.Errorf("mock hasher failure")
	}

	// Calculate real totals
	var totalRobo, totalSale float64
	for _, ingot := range batch {
		totalRobo += ingot.RoboTorq
		totalSale += ingot.Price
	}

	// Return deterministic hash
	return "mock_hash_" + fmt.Sprint(len(batch)), totalRobo, totalSale, nil
}

func (m *mockBatchHasher) getHashCalls() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.hashCalls
}

// mockDistoDamClient is a test double for DistoDamClient.
type mockDistoDamClient struct {
	publishedEvents []*MintEvent
	shouldFail      bool
	isConnected     bool
	mu              sync.Mutex
}

func (m *mockDistoDamClient) Publish(ctx context.Context, event *MintEvent) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.shouldFail {
		return fmt.Errorf("mock publish failure")
	}

	m.publishedEvents = append(m.publishedEvents, event)
	return nil
}

func (m *mockDistoDamClient) Connect() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.isConnected = true
	return nil
}

func (m *mockDistoDamClient) Close() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.isConnected = false
	return nil
}

func (m *mockDistoDamClient) IsConnected() bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.isConnected
}

func (m *mockDistoDamClient) getPublishedEvents() []*MintEvent {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.publishedEvents
}

// setupTestMintEngine creates a test MintEngine with mocks.
func setupTestMintEngine() (MintEngine, *mockBatchHasher, *mockDistoDamClient) {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	hasher := &mockBatchHasher{}
	client := &mockDistoDamClient{isConnected: true}
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	engine := NewMintEngine(hasher, client, logger)

	return engine, hasher, client
}

// ─────────────────────────────────────────────────────────────
// Basic Processing Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_ProcessBatch_Success(t *testing.T) {
	engine, hasher, client := setupTestMintEngine()

	batch := []*TokenTorqIngot{
		{JouleTorq: 3600, RoboTorq: 100.0, Price: 50.0, Hash: "h1"},
		{JouleTorq: 3600, RoboTorq: 200.0, Price: 75.0, Hash: "h2"},
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)

	require.NoError(t, err)
	assert.Equal(t, 1, hasher.getHashCalls())

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]
	assert.Equal(t, "mock_hash_2", event.BatchHash)
	assert.Equal(t, 300.0, event.TotalRoboTorq) // 100 + 200
	assert.Equal(t, 125.0, event.SaleValueUSD)  // 50 + 75
	assert.Equal(t, 2, event.IngotsProcessed)
	assert.NotEmpty(t, event.BatchID)
	assert.WithinDuration(t, time.Now(), event.Timestamp, 1*time.Second)
}

func TestMintEngine_ProcessBatch_EmptyBatch(t *testing.T) {
	engine, _, _ := setupTestMintEngine()

	batch := []*TokenTorqIngot{}
	ctx := context.Background()

	err := engine.ProcessBatch(ctx, batch)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "empty batch")
}

func TestMintEngine_ProcessBatch_MultipleBatches(t *testing.T) {
	engine, hasher, client := setupTestMintEngine()

	ctx := context.Background()

	// Process 3 batches
	for i := 0; i < 3; i++ {
		batch := []*TokenTorqIngot{
			{RoboTorq: float64(i * 100), Price: float64(i * 50), Hash: fmt.Sprintf("h%d", i)},
		}
		err := engine.ProcessBatch(ctx, batch)
		require.NoError(t, err)
	}

	assert.Equal(t, 3, hasher.getHashCalls())
	assert.Len(t, client.getPublishedEvents(), 3)
}

// ─────────────────────────────────────────────────────────────
// Error Handling Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_ProcessBatch_HasherFailure(t *testing.T) {
	engine, hasher, client := setupTestMintEngine()

	hasher.shouldFail = true

	batch := []*TokenTorqIngot{
		{RoboTorq: 100.0, Price: 50.0, Hash: "h1"},
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "hashing failed")
	assert.Equal(t, 1, hasher.getHashCalls())
	assert.Len(t, client.getPublishedEvents(), 0) // Should not publish on hash failure
}

func TestMintEngine_ProcessBatch_PublishFailure(t *testing.T) {
	engine, hasher, client := setupTestMintEngine()

	client.shouldFail = true

	batch := []*TokenTorqIngot{
		{RoboTorq: 100.0, Price: 50.0, Hash: "h1"},
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "failed to publish")
	assert.Equal(t, 1, hasher.getHashCalls()) // Hash should still be called
}

// ─────────────────────────────────────────────────────────────
// Metrics Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_GetTotalProcessed(t *testing.T) {
	engine, _, _ := setupTestMintEngine()

	ctx := context.Background()

	// Initially zero
	assert.Equal(t, int64(0), engine.GetTotalProcessed())

	// Process batches of different sizes
	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		{RoboTorq: 100, Price: 50, Hash: "h1"},
		{RoboTorq: 200, Price: 75, Hash: "h2"},
	})

	assert.Equal(t, int64(2), engine.GetTotalProcessed())

	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		{RoboTorq: 100, Price: 50, Hash: "h3"},
		{RoboTorq: 200, Price: 75, Hash: "h4"},
		{RoboTorq: 300, Price: 100, Hash: "h5"},
	})

	assert.Equal(t, int64(5), engine.GetTotalProcessed())
}

func TestMintEngine_GetTotalRoboAggregated(t *testing.T) {
	engine, _, _ := setupTestMintEngine()

	ctx := context.Background()

	// Initially zero
	assert.Equal(t, 0.0, engine.GetTotalRoboAggregated())

	// Process batch 1
	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		{RoboTorq: 123.45, Price: 50, Hash: "h1"},
		{RoboTorq: 678.90, Price: 75, Hash: "h2"},
	})

	assert.InDelta(t, 802.35, engine.GetTotalRoboAggregated(), 0.01)

	// Process batch 2
	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		{RoboTorq: 234.56, Price: 100, Hash: "h3"},
	})

	assert.InDelta(t, 1036.91, engine.GetTotalRoboAggregated(), 0.01)
}

// ─────────────────────────────────────────────────────────────
// MintEvent Generation Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_MintEvent_Structure(t *testing.T) {
	engine, _, client := setupTestMintEngine()

	batch := []*TokenTorqIngot{
		{JouleTorq: 3600, RoboTorq: 500.0, Price: 250.0, Hash: "h1"},
		{JouleTorq: 3600, RoboTorq: 600.0, Price: 300.0, Hash: "h2"},
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)
	require.NoError(t, err)

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]

	// Verify all fields are populated
	assert.NotEmpty(t, event.BatchHash)
	assert.Equal(t, 1100.0, event.TotalRoboTorq)
	assert.Equal(t, 550.0, event.SaleValueUSD)
	assert.Equal(t, 2, event.IngotsProcessed)
	assert.NotEmpty(t, event.BatchID)
	assert.False(t, event.Timestamp.IsZero())

	// BatchID should be valid UUID format (36 chars with hyphens)
	assert.Len(t, event.BatchID, 36)
	assert.Contains(t, event.BatchID, "-")
}

func TestMintEngine_MintEvent_UniqueBatchIDs(t *testing.T) {
	engine, _, client := setupTestMintEngine()

	ctx := context.Background()

	// Process 5 batches
	for i := 0; i < 5; i++ {
		batch := []*TokenTorqIngot{
			{RoboTorq: float64(i * 100), Price: float64(i * 50), Hash: fmt.Sprintf("h%d", i)},
		}
		err := engine.ProcessBatch(ctx, batch)
		require.NoError(t, err)
	}

	events := client.getPublishedEvents()
	require.Len(t, events, 5)

	// All BatchIDs should be unique
	batchIDs := make(map[string]bool)
	for _, event := range events {
		assert.False(t, batchIDs[event.BatchID], "Duplicate BatchID found")
		batchIDs[event.BatchID] = true
	}
}

// ─────────────────────────────────────────────────────────────
// Concurrent Processing Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_ConcurrentProcessing(t *testing.T) {
	engine, _, client := setupTestMintEngine()

	ctx := context.Background()

	// Process 10 batches concurrently
	var wg sync.WaitGroup
	numBatches := 10

	wg.Add(numBatches)
	for i := 0; i < numBatches; i++ {
		go func(batchNum int) {
			defer wg.Done()
			batch := []*TokenTorqIngot{
				{RoboTorq: float64(batchNum * 100), Price: 50.0, Hash: fmt.Sprintf("h%d", batchNum)},
			}
			err := engine.ProcessBatch(ctx, batch)
			assert.NoError(t, err)
		}(i)
	}

	wg.Wait()

	// Should have published 10 events
	events := client.getPublishedEvents()
	assert.Equal(t, numBatches, len(events))

	// Metrics should be accurate
	assert.Equal(t, int64(numBatches), engine.GetTotalProcessed())
}

// ─────────────────────────────────────────────────────────────
// Integration with SimpleBatchHasher Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_WithRealHasher(t *testing.T) {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	hasher := NewSimpleBatchHasher()
	client := &mockDistoDamClient{isConnected: true}
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	engine := NewMintEngine(hasher, client, logger)

	batch := []*TokenTorqIngot{
		{
			JouleTorq:  3600,
			RoboTorq:   100.0,
			Price:      50.0,
			Hash:       "hash1",
			ContractID: "contract1",
			DiggerID:   "digger1",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
		{
			JouleTorq:  3600,
			RoboTorq:   200.0,
			Price:      75.0,
			Hash:       "hash2",
			ContractID: "contract2",
			DiggerID:   "digger2",
			Timestamp:  time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		},
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)
	require.NoError(t, err)

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]

	// Real hasher should produce 64-char hex hash
	assert.Len(t, event.BatchHash, 64)
	assert.Regexp(t, "^[0-9a-f]{64}$", event.BatchHash)

	// Totals should match
	assert.Equal(t, 300.0, event.TotalRoboTorq)
	assert.Equal(t, 125.0, event.SaleValueUSD)
	assert.Equal(t, 2, event.IngotsProcessed)
}

func TestMintEngine_WithRealHasher_LargeBatch(t *testing.T) {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	hasher := NewSimpleBatchHasher()
	client := &mockDistoDamClient{isConnected: true}
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	engine := NewMintEngine(hasher, client, logger)

	// Create 1000-ingot batch
	batch := make([]*TokenTorqIngot, 1000)
	expectedRobo := 0.0
	expectedSale := 0.0

	for i := 0; i < 1000; i++ {
		robo := float64(i)
		price := float64(i) * 0.5
		batch[i] = &TokenTorqIngot{
			JouleTorq:  3600,
			RoboTorq:   robo,
			Price:      price,
			Hash:       fmt.Sprintf("hash%d", i),
			ContractID: "contract",
			DiggerID:   "digger",
			Timestamp:  time.Now(),
		}
		expectedRobo += robo
		expectedSale += price
	}

	ctx := context.Background()
	start := time.Now()

	err := engine.ProcessBatch(ctx, batch)

	elapsed := time.Since(start)
	require.NoError(t, err)

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]
	assert.InDelta(t, expectedRobo, event.TotalRoboTorq, 0.01)
	assert.InDelta(t, expectedSale, event.SaleValueUSD, 0.01)
	assert.Equal(t, 1000, event.IngotsProcessed)

	t.Logf("Processed 1000-ingot batch in %v", elapsed)
	assert.Less(t, elapsed, 50*time.Millisecond, "Should process 1000 ingots in under 50ms")
}

// ─────────────────────────────────────────────────────────────
// Context Cancellation Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_ContextCancellation(t *testing.T) {
	engine, _, _ := setupTestMintEngine()

	batch := []*TokenTorqIngot{
		{RoboTorq: 100.0, Price: 50.0, Hash: "h1"},
	}

	// Create cancelled context
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	// ProcessBatch should still complete (context mainly affects publish)
	// In this test, mock client doesn't check context
	err := engine.ProcessBatch(ctx, batch)

	// Depending on implementation, this might succeed or fail
	// For now, we just verify it doesn't panic
	t.Logf("ProcessBatch with cancelled context: %v", err)
}
