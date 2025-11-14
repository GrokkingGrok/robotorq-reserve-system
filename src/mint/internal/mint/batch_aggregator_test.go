package mint

import (
	"context"
	"io"
	"log/slog"
	"sync"
	"testing"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/testutil"
	"github.com/stretchr/testify/assert"
)

// ─────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────

// mockMintEngine is a test double for MintEngine.
type mockMintEngine struct {
	processedBatches [][]*TokenTorqIngot
	processCount     int
	totalIngots      int64
	totalRobo        float64
	processingDelay  time.Duration // Simulate processing time
	shouldFail       bool          // Simulate processing failures
	mu               sync.Mutex
}

func newMockMintEngine() *mockMintEngine {
	return &mockMintEngine{
		processedBatches: make([][]*TokenTorqIngot, 0),
	}
}

func (m *mockMintEngine) ProcessBatch(ctx context.Context, batch []*TokenTorqIngot) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.shouldFail {
		return assert.AnError
	}

	// Simulate processing delay
	if m.processingDelay > 0 {
		time.Sleep(m.processingDelay)
	}

	// Store batch copy
	batchCopy := make([]*TokenTorqIngot, len(batch))
	copy(batchCopy, batch)
	m.processedBatches = append(m.processedBatches, batchCopy)

	m.processCount++
	m.totalIngots += int64(len(batch))

	for _, ingot := range batch {
		m.totalRobo += ingot.RoboTorq
	}

	return nil
}

func (m *mockMintEngine) GetTotalProcessed() int64 {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.totalIngots
}

func (m *mockMintEngine) GetTotalRoboAggregated() float64 {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.totalRobo
}

func (m *mockMintEngine) getBatchCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.processCount
}

// setupTestAggregator creates a test BatchAggregator with mocks.
func setupTestAggregator(batchSize int, flushInterval time.Duration) (BatchAggregator, IngotBuffer, *mockMintEngine) {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	buffer := NewIngotBuffer(10000, slog.New(slog.NewTextHandler(io.Discard, nil)))
	engine := newMockMintEngine()
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	aggregator := NewBatchAggregator(buffer, engine, batchSize, flushInterval, logger)

	return aggregator, buffer, engine
}

// ─────────────────────────────────────────────────────────────
// Threshold Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_ThresholdTriggering(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(10, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start aggregator
	go aggregator.Start(ctx)

	// Push exactly 10 ingots (threshold)
	for i := 0; i < 10; i++ {
		buffer.Push(validIngot())
	}

	// Wait for processing
	time.Sleep(100 * time.Millisecond)

	// Should have processed exactly 1 batch
	assert.Equal(t, 1, engine.getBatchCount())
	assert.Equal(t, int64(10), engine.GetTotalProcessed())
	assert.Equal(t, 0, aggregator.GetAccumulatedCount())
}

func TestBatchAggregator_MultipleFullBatches(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(5, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push 17 ingots (3 full batches + 2 remainder)
	for i := 0; i < 17; i++ {
		buffer.Push(validIngot())
	}

	// Wait for processing
	time.Sleep(100 * time.Millisecond)

	// Should have processed 3 full batches
	assert.Equal(t, 3, engine.getBatchCount())
	assert.Equal(t, int64(15), engine.GetTotalProcessed())
	assert.Equal(t, 2, aggregator.GetAccumulatedCount()) // 2 remaining
}

// ─────────────────────────────────────────────────────────────
// Interval Flush Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_IntervalFlush(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(100, 200*time.Millisecond)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push 5 ingots (less than threshold)
	for i := 0; i < 5; i++ {
		buffer.Push(validIngot())
	}

	// Wait for interval flush
	time.Sleep(300 * time.Millisecond)

	// Should have flushed partial batch
	assert.Equal(t, 1, engine.getBatchCount())
	assert.Equal(t, int64(5), engine.GetTotalProcessed())
	assert.Equal(t, 0, aggregator.GetAccumulatedCount())
}

func TestBatchAggregator_NoFlushWhenEmpty(t *testing.T) {
	aggregator, _, engine := setupTestAggregator(10, 100*time.Millisecond)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Wait for multiple flush intervals
	time.Sleep(300 * time.Millisecond)

	// Should not have processed any batches (buffer empty)
	assert.Equal(t, 0, engine.getBatchCount())
	assert.Equal(t, int64(0), engine.GetTotalProcessed())
}

// ─────────────────────────────────────────────────────────────
// Explicit Flush Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_ExplicitFlush(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(100, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push some ingots (less than threshold)
	for i := 0; i < 7; i++ {
		buffer.Push(validIngot())
	}

	time.Sleep(50 * time.Millisecond)

	// Explicit flush
	err := aggregator.Flush()
	assert.NoError(t, err)

	// Should have flushed
	assert.Equal(t, 1, engine.getBatchCount())
	assert.Equal(t, int64(7), engine.GetTotalProcessed())
	assert.Equal(t, 0, aggregator.GetAccumulatedCount())
}

func TestBatchAggregator_FlushEmpty(t *testing.T) {
	aggregator, _, engine := setupTestAggregator(10, 1*time.Hour)

	// Flush without any ingots
	err := aggregator.Flush()
	assert.NoError(t, err)

	// Should not have processed anything
	assert.Equal(t, 0, engine.getBatchCount())
}

// ─────────────────────────────────────────────────────────────
// Carryover Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_CarryoverBetweenBatches(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(10, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push 23 ingots (2 full batches + 3 carryover)
	for i := 0; i < 23; i++ {
		ingot := validIngot()
		ingot.ContractID = "ingot-" + string(rune('A'+i))
		buffer.Push(ingot)
	}

	time.Sleep(100 * time.Millisecond)

	// Should have processed 2 batches
	assert.Equal(t, 2, engine.getBatchCount())
	assert.Equal(t, 3, aggregator.GetAccumulatedCount())

	// Verify batch contents
	engine.mu.Lock()
	assert.Equal(t, 2, len(engine.processedBatches))
	assert.Equal(t, 10, len(engine.processedBatches[0]))
	assert.Equal(t, 10, len(engine.processedBatches[1]))

	// First batch should be ingots 0-9
	assert.Equal(t, "ingot-A", engine.processedBatches[0][0].ContractID)
	assert.Equal(t, "ingot-J", engine.processedBatches[0][9].ContractID)

	// Second batch should be ingots 10-19
	assert.Equal(t, "ingot-K", engine.processedBatches[1][0].ContractID)
	assert.Equal(t, "ingot-T", engine.processedBatches[1][9].ContractID)
	engine.mu.Unlock()
}

// ─────────────────────────────────────────────────────────────
// Shutdown Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_ShutdownFlush(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(100, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())

	go aggregator.Start(ctx)

	// Push partial batch
	for i := 0; i < 12; i++ {
		buffer.Push(validIngot())
	}

	time.Sleep(50 * time.Millisecond)

	// Shutdown: cancel context then flush
	cancel()
	time.Sleep(50 * time.Millisecond)

	err := aggregator.Flush()
	assert.NoError(t, err)

	// Should have flushed partial batch
	assert.Equal(t, 1, engine.getBatchCount())
	assert.Equal(t, int64(12), engine.GetTotalProcessed())
}

func TestBatchAggregator_NoDataLossOnShutdown(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(10, 100*time.Millisecond)

	ctx, cancel := context.WithCancel(context.Background())

	go aggregator.Start(ctx)

	// Push 47 ingots (4 full + 7 partial)
	for i := 0; i < 47; i++ {
		buffer.Push(validIngot())
	}

	time.Sleep(200 * time.Millisecond)

	// Should have processed at least 4 full batches (may have processed 5 due to interval)
	assert.GreaterOrEqual(t, engine.getBatchCount(), 4)

	// Shutdown and flush
	cancel()
	time.Sleep(50 * time.Millisecond)
	aggregator.Flush()

	// All ingots should be processed (total 47)
	assert.Equal(t, int64(47), engine.GetTotalProcessed())
	assert.Equal(t, 0, aggregator.GetAccumulatedCount())
} // ─────────────────────────────────────────────────────────────
// Metrics Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_MetricsTracking(t *testing.T) {
	aggregator, buffer, _ := setupTestAggregator(10, 200*time.Millisecond)
	agg := aggregator.(*batchAggregator)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Trigger full batch (threshold)
	for i := 0; i < 10; i++ {
		buffer.Push(validIngot())
	}
	time.Sleep(100 * time.Millisecond)

	// Check full batch metric
	assert.Equal(t, 1.0, testutil.ToFloat64(agg.metrics.fullBatches))
	assert.Equal(t, 1.0, testutil.ToFloat64(agg.metrics.batchesProcessed))

	// Trigger partial batch (interval) - push less than threshold
	for i := 0; i < 3; i++ {
		buffer.Push(validIngot())
	}
	time.Sleep(300 * time.Millisecond) // Wait longer for interval flush

	// Check partial flush metric
	assert.GreaterOrEqual(t, testutil.ToFloat64(agg.metrics.partialFlushes), 1.0)
	assert.GreaterOrEqual(t, testutil.ToFloat64(agg.metrics.batchesProcessed), 2.0)

	// Shutdown without adding more (to avoid triggering another interval flush)
	cancel()
} // ─────────────────────────────────────────────────────────────
// Edge Cases
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_ProcessingFailure(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(5, 1*time.Hour)
	engine.shouldFail = true

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push full batch
	for i := 0; i < 5; i++ {
		buffer.Push(validIngot())
	}

	time.Sleep(100 * time.Millisecond)

	// Batch should have been attempted but failed
	// Current implementation logs error and continues
	assert.Equal(t, 0, aggregator.GetAccumulatedCount())
}

func TestBatchAggregator_SlowProcessing(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(5, 1*time.Hour)
	engine.processingDelay = 50 * time.Millisecond

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push 2 full batches
	for i := 0; i < 10; i++ {
		buffer.Push(validIngot())
	}

	// Wait for both to complete (with processing delay)
	time.Sleep(200 * time.Millisecond)
	assert.Equal(t, 2, engine.getBatchCount())
} // ─────────────────────────────────────────────────────────────
// High Throughput Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_HighThroughput(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping stress test in short mode")
	}

	aggregator, buffer, engine := setupTestAggregator(1000, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Push 10,000 ingots rapidly
	numIngots := 10000
	start := time.Now()

	for i := 0; i < numIngots; i++ {
		buffer.Push(validIngot())
	}

	// Wait for all batches to process
	time.Sleep(500 * time.Millisecond)

	duration := time.Since(start)

	// Should have processed 10 full batches
	assert.Equal(t, 10, engine.getBatchCount())
	assert.Equal(t, int64(10000), engine.GetTotalProcessed())

	t.Logf("Processed %d ingots in %v (%.0f ingots/sec)",
		numIngots, duration, float64(numIngots)/duration.Seconds())
}

func TestBatchAggregator_ConcurrentBufferFill(t *testing.T) {
	aggregator, buffer, engine := setupTestAggregator(100, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	// Multiple goroutines pushing to buffer
	var wg sync.WaitGroup
	numGoroutines := 5
	ingotsPerGoroutine := 50

	wg.Add(numGoroutines)
	for i := 0; i < numGoroutines; i++ {
		go func() {
			defer wg.Done()
			for j := 0; j < ingotsPerGoroutine; j++ {
				buffer.Push(validIngot())
			}
		}()
	}

	wg.Wait()
	time.Sleep(500 * time.Millisecond) // Give aggregator time to process

	// Should have processed all ingots (at least 2 full batches = 200)
	// The remaining 50 may or may not flush depending on timing
	totalIngots := numGoroutines * ingotsPerGoroutine
	assert.GreaterOrEqual(t, engine.GetTotalProcessed(), int64(200))
	assert.LessOrEqual(t, engine.GetTotalProcessed(), int64(totalIngots))
} // ─────────────────────────────────────────────────────────────
// Context Cancellation Tests
// ─────────────────────────────────────────────────────────────

func TestBatchAggregator_ContextCancellation(t *testing.T) {
	aggregator, _, _ := setupTestAggregator(10, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())

	// Start and immediately cancel
	errChan := make(chan error, 1)
	go func() {
		errChan <- aggregator.Start(ctx)
	}()

	cancel()

	// Should return context.Canceled
	select {
	case err := <-errChan:
		assert.ErrorIs(t, err, context.Canceled)
	case <-time.After(1 * time.Second):
		t.Fatal("Start() did not return after context cancellation")
	}
}

func TestBatchAggregator_GetAccumulatedCount(t *testing.T) {
	aggregator, buffer, _ := setupTestAggregator(100, 1*time.Hour)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go aggregator.Start(ctx)

	assert.Equal(t, 0, aggregator.GetAccumulatedCount())

	// Add ingots
	for i := 0; i < 7; i++ {
		buffer.Push(validIngot())
	}

	time.Sleep(50 * time.Millisecond)

	assert.Equal(t, 7, aggregator.GetAccumulatedCount())
}
