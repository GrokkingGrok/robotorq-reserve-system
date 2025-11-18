package mint

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"sync"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
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

func (m *mockBatchHasher) Hash(batch []*TokenTorqIngot) (string, []IngotStake, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.hashCalls++

	if m.shouldFail {
		return "", nil, fmt.Errorf("mock hasher failure")
	}

	// Build IngotStakes array (Phase 6: Preserve granularity)
	ingotStakes := make([]IngotStake, len(batch))
	for i, ingot := range batch {
		ingotStakes[i] = IngotStake{
			IngotID:        ingot.IngotID,
			RoboStakeTotal: ingot.RoboStakeTotal,
			ContractIDs:    ingot.ContractIDs,
		}
	}

	// Return deterministic hash
	return "mock_hash_" + fmt.Sprint(len(batch)), ingotStakes, nil
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

func (m *mockDistoDamClient) GetConnection() *nats.Conn {
	return nil // Mock doesn't need real NATS connection
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
		createStubIngotForMintTests("i1", "c1", 3600, 100.0),
		createStubIngotForMintTests("i2", "c2", 3600, 200.0),
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)

	require.NoError(t, err)
	assert.Equal(t, 1, hasher.getHashCalls())

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]
	assert.Equal(t, "mock_hash_2", event.BatchHash)
	// Phase 6: Check IngotStakes array instead of summed total
	require.Len(t, event.IngotStakes, 2)
	assert.Equal(t, 100.0, event.IngotStakes[0].RoboStakeTotal)
	assert.Equal(t, 200.0, event.IngotStakes[1].RoboStakeTotal)
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
			createStubIngotForMintTests(fmt.Sprintf("i%d", i), fmt.Sprintf("c%d", i), 3600, float64(i*100)),
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
		createStubIngotForMintTests("i1", "c1", 3600, 100.0),
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
		createStubIngotForMintTests("i1", "c1", 3600, 100.0),
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
		createStubIngotForMintTests("i1", "c1", 3600, 100.0),
		createStubIngotForMintTests("i2", "c2", 3600, 200.0),
	})

	assert.Equal(t, int64(2), engine.GetTotalProcessed())

	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		createStubIngotForMintTests("i3", "c3", 3600, 100.0),
		createStubIngotForMintTests("i4", "c4", 3600, 200.0),
		createStubIngotForMintTests("i5", "c5", 3600, 300.0),
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
		createStubIngotForMintTests("i1", "c1", 3600, 123.45),
		createStubIngotForMintTests("i2", "c2", 3600, 678.90),
	})

	assert.InDelta(t, 802.35, engine.GetTotalRoboAggregated(), 0.01)

	// Process batch 2
	engine.ProcessBatch(ctx, []*TokenTorqIngot{
		createStubIngotForMintTests("i3", "c3", 3600, 234.56),
	})

	assert.InDelta(t, 1036.91, engine.GetTotalRoboAggregated(), 0.01)
}

// ─────────────────────────────────────────────────────────────
// MintEvent Generation Tests
// ─────────────────────────────────────────────────────────────

func TestMintEngine_MintEvent_Structure(t *testing.T) {
	engine, _, client := setupTestMintEngine()

	batch := []*TokenTorqIngot{
		createStubIngotForMintTests("i1", "c1", 3600, 500.0),
		createStubIngotForMintTests("i2", "c2", 3600, 600.0),
	}

	ctx := context.Background()
	err := engine.ProcessBatch(ctx, batch)
	require.NoError(t, err)

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]

	// Verify all fields are populated
	assert.NotEmpty(t, event.BatchHash)
	// Phase 6: Check IngotStakes array
	require.Len(t, event.IngotStakes, 2)
	assert.Equal(t, 500.0, event.IngotStakes[0].RoboStakeTotal)
	assert.Equal(t, 600.0, event.IngotStakes[1].RoboStakeTotal)
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
			createStubIngotForMintTests(fmt.Sprintf("i%d", i), fmt.Sprintf("c%d", i), 3600, float64(i*100)),
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
				createStubIngotForMintTests(fmt.Sprintf("i%d", batchNum), fmt.Sprintf("c%d", batchNum), 3600, float64(batchNum*100)),
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
		createStubIngotForMintTests("i1", "contract1", 3600, 100.0),
		createStubIngotForMintTests("i2", "contract2", 3600, 200.0),
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

	// Phase 6: Check IngotStakes array
	require.Len(t, event.IngotStakes, 2)
	assert.Equal(t, 100.0, event.IngotStakes[0].RoboStakeTotal)
	assert.Equal(t, 200.0, event.IngotStakes[1].RoboStakeTotal)
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

	for i := 0; i < 1000; i++ {
		robo := float64(i)
		batch[i] = createStubIngotForMintTests(fmt.Sprintf("i%d", i), "contract", 3600, robo)
		expectedRobo += robo
	}

	ctx := context.Background()
	start := time.Now()

	err := engine.ProcessBatch(ctx, batch)

	elapsed := time.Since(start)
	require.NoError(t, err)

	events := client.getPublishedEvents()
	require.Len(t, events, 1)

	event := events[0]
	// Phase 6: Verify IngotStakes array has 1000 entries
	require.Len(t, event.IngotStakes, 1000)
	// Calculate total from stakes
	var totalFromStakes float64
	for _, stake := range event.IngotStakes {
		totalFromStakes += stake.RoboStakeTotal
	}
	assert.InDelta(t, expectedRobo, totalFromStakes, 0.01)
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
		createStubIngotForMintTests("i1", "c1", 3600, 100.0),
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
