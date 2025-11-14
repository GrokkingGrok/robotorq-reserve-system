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
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────

// setupTestBuffer creates a test IngotBuffer with specified capacity.
func setupTestBuffer(capacity int) IngotBuffer {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	return NewIngotBuffer(capacity, logger)
}

// ─────────────────────────────────────────────────────────────
// Basic Operations Tests
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_PushSuccess(t *testing.T) {
	buffer := setupTestBuffer(10)

	ingot := validIngot()
	err := buffer.Push(ingot)
	assert.NoError(t, err)
	assert.Equal(t, 1, buffer.Len())
}

func TestIngotBuffer_PushMultiple(t *testing.T) {
	buffer := setupTestBuffer(10)

	for i := 0; i < 5; i++ {
		err := buffer.Push(validIngot())
		assert.NoError(t, err)
	}

	assert.Equal(t, 5, buffer.Len())
}

func TestIngotBuffer_PushFull(t *testing.T) {
	buffer := setupTestBuffer(3)

	// Fill buffer
	assert.NoError(t, buffer.Push(validIngot()))
	assert.NoError(t, buffer.Push(validIngot()))
	assert.NoError(t, buffer.Push(validIngot()))
	assert.Equal(t, 3, buffer.Len())

	// Try to push when full
	err := buffer.Push(validIngot())
	assert.ErrorIs(t, err, ErrBufferFull)
	assert.Equal(t, 3, buffer.Len()) // Still at capacity
}

func TestIngotBuffer_PopSuccess(t *testing.T) {
	buffer := setupTestBuffer(10)
	ctx := context.Background()

	original := validIngot()
	original.ContractID = "test-123"

	buffer.Push(original)

	popped, err := buffer.Pop(ctx)
	assert.NoError(t, err)
	assert.NotNil(t, popped)
	assert.Equal(t, "test-123", popped.ContractID)
	assert.Equal(t, 0, buffer.Len())
}

func TestIngotBuffer_PopEmpty(t *testing.T) {
	buffer := setupTestBuffer(10)
	ctx, cancel := context.WithTimeout(context.Background(), 100*time.Millisecond)
	defer cancel()

	// Pop from empty buffer should block until context timeout
	_, err := buffer.Pop(ctx)
	assert.Error(t, err)
	assert.ErrorIs(t, err, context.DeadlineExceeded)
}

func TestIngotBuffer_PopContextCancelled(t *testing.T) {
	buffer := setupTestBuffer(10)
	ctx, cancel := context.WithCancel(context.Background())

	// Cancel immediately
	cancel()

	_, err := buffer.Pop(ctx)
	assert.Error(t, err)
	assert.ErrorIs(t, err, context.Canceled)
}

func TestIngotBuffer_LenCap(t *testing.T) {
	buffer := setupTestBuffer(100)

	assert.Equal(t, 0, buffer.Len())
	assert.Equal(t, 100, buffer.Cap())

	buffer.Push(validIngot())
	buffer.Push(validIngot())

	assert.Equal(t, 2, buffer.Len())
	assert.Equal(t, 100, buffer.Cap())
}

func TestIngotBuffer_Drain(t *testing.T) {
	buffer := setupTestBuffer(10)

	// Add some ingots
	for i := 0; i < 5; i++ {
		buffer.Push(validIngot())
	}
	assert.Equal(t, 5, buffer.Len())

	// Drain
	ingots := buffer.Drain()
	assert.Equal(t, 5, len(ingots))
	assert.Equal(t, 0, buffer.Len())
}

func TestIngotBuffer_DrainEmpty(t *testing.T) {
	buffer := setupTestBuffer(10)

	ingots := buffer.Drain()
	assert.Equal(t, 0, len(ingots))
	assert.Equal(t, 0, buffer.Len())
}

// ─────────────────────────────────────────────────────────────
// Concurrency Tests
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_ConcurrentPush(t *testing.T) {
	buffer := setupTestBuffer(1000)
	numGoroutines := 10
	ingotsPerGoroutine := 50

	var wg sync.WaitGroup
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

	expected := numGoroutines * ingotsPerGoroutine
	assert.Equal(t, expected, buffer.Len())
}

func TestIngotBuffer_ConcurrentPop(t *testing.T) {
	buffer := setupTestBuffer(1000)
	numIngots := 500

	// Fill buffer
	for i := 0; i < numIngots; i++ {
		buffer.Push(validIngot())
	}

	numGoroutines := 10
	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	ctx := context.Background()
	ingotsPerGoroutine := numIngots / numGoroutines

	for i := 0; i < numGoroutines; i++ {
		go func() {
			defer wg.Done()
			for j := 0; j < ingotsPerGoroutine; j++ {
				_, err := buffer.Pop(ctx)
				assert.NoError(t, err)
			}
		}()
	}

	wg.Wait()

	assert.Equal(t, 0, buffer.Len())
}

func TestIngotBuffer_ConcurrentPushPop(t *testing.T) {
	buffer := setupTestBuffer(100)
	numOperations := 200
	ctx := context.Background()

	var wg sync.WaitGroup
	wg.Add(2)

	// Pusher goroutine
	go func() {
		defer wg.Done()
		for i := 0; i < numOperations; i++ {
			for {
				err := buffer.Push(validIngot())
				if err == nil {
					break
				}
				// If buffer full, wait a bit and retry
				time.Sleep(1 * time.Millisecond)
			}
		}
	}()

	// Popper goroutine
	poppedCount := 0
	go func() {
		defer wg.Done()
		for i := 0; i < numOperations; i++ {
			_, err := buffer.Pop(ctx)
			if err == nil {
				poppedCount++
			}
		}
	}()

	wg.Wait()

	// All pushed items should have been popped
	assert.Equal(t, numOperations, poppedCount)
	assert.Equal(t, 0, buffer.Len())
}

// ─────────────────────────────────────────────────────────────
// Metrics Tests
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_MetricsUpdated(t *testing.T) {
	buffer := setupTestBuffer(10)
	b := buffer.(*ingotBuffer)
	ctx := context.Background()

	// Initial state
	assert.Equal(t, 0.0, testutil.ToFloat64(b.metrics.ingotsBuffered))

	// Push ingots
	buffer.Push(validIngot())
	buffer.Push(validIngot())
	buffer.Push(validIngot())

	assert.Equal(t, 3.0, testutil.ToFloat64(b.metrics.ingotsBuffered))
	assert.Equal(t, 3.0, testutil.ToFloat64(b.metrics.pushCount))

	// Pop ingot
	buffer.Pop(ctx)

	assert.Equal(t, 2.0, testutil.ToFloat64(b.metrics.ingotsBuffered))
	assert.Equal(t, 1.0, testutil.ToFloat64(b.metrics.popCount))

	// Drain
	buffer.Drain()

	assert.Equal(t, 0.0, testutil.ToFloat64(b.metrics.ingotsBuffered))
	assert.Equal(t, 1.0, testutil.ToFloat64(b.metrics.drainCount))
}

func TestIngotBuffer_AtomicCounterAccuracy(t *testing.T) {
	buffer := setupTestBuffer(1000)
	numGoroutines := 20
	opsPerGoroutine := 50

	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	// Concurrent pushes
	for i := 0; i < numGoroutines; i++ {
		go func() {
			defer wg.Done()
			for j := 0; j < opsPerGoroutine; j++ {
				buffer.Push(validIngot())
			}
		}()
	}

	wg.Wait()

	expectedCount := numGoroutines * opsPerGoroutine
	assert.Equal(t, expectedCount, buffer.Len())

	// Verify atomic counter matches channel length
	b := buffer.(*ingotBuffer)
	assert.Equal(t, int64(expectedCount), b.depth.Load())
}

// ─────────────────────────────────────────────────────────────
// FIFO Order Tests
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_FIFOOrder(t *testing.T) {
	buffer := setupTestBuffer(10)
	ctx := context.Background()

	// Push ingots with unique IDs
	ids := []string{"first", "second", "third"}
	for _, id := range ids {
		ingot := validIngot()
		ingot.ContractID = id
		buffer.Push(ingot)
	}

	// Pop and verify order
	for _, expectedID := range ids {
		ingot, err := buffer.Pop(ctx)
		require.NoError(t, err)
		assert.Equal(t, expectedID, ingot.ContractID)
	}
}

// ─────────────────────────────────────────────────────────────
// Graceful Shutdown Scenario
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_GracefulShutdown(t *testing.T) {
	buffer := setupTestBuffer(100)

	// Simulate production scenario: ingots arrive
	for i := 0; i < 25; i++ {
		ingot := validIngot()
		ingot.ContractID = "contract-" + string(rune('A'+i))
		buffer.Push(ingot)
	}

	assert.Equal(t, 25, buffer.Len())

	// Shutdown signal received - drain buffer
	drained := buffer.Drain()

	assert.Equal(t, 25, len(drained))
	assert.Equal(t, 0, buffer.Len())

	// Verify all ingots preserved
	for i, ingot := range drained {
		expected := "contract-" + string(rune('A'+i))
		assert.Equal(t, expected, ingot.ContractID)
	}
}

// ─────────────────────────────────────────────────────────────
// Stress Tests
// ─────────────────────────────────────────────────────────────

func TestIngotBuffer_HighThroughput(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping stress test in short mode")
	}

	buffer := setupTestBuffer(10000)
	numIngots := 5000
	ctx := context.Background()

	// Fill buffer quickly
	start := time.Now()
	for i := 0; i < numIngots; i++ {
		buffer.Push(validIngot())
	}
	pushDuration := time.Since(start)

	t.Logf("Pushed %d ingots in %v (%.0f ingots/sec)",
		numIngots, pushDuration, float64(numIngots)/pushDuration.Seconds())

	assert.Equal(t, numIngots, buffer.Len())

	// Drain buffer quickly
	start = time.Now()
	for i := 0; i < numIngots; i++ {
		_, err := buffer.Pop(ctx)
		assert.NoError(t, err)
	}
	popDuration := time.Since(start)

	t.Logf("Popped %d ingots in %v (%.0f ingots/sec)",
		numIngots, popDuration, float64(numIngots)/popDuration.Seconds())

	assert.Equal(t, 0, buffer.Len())
}

func TestIngotBuffer_BackpressureRecovery(t *testing.T) {
	buffer := setupTestBuffer(5)

	// Fill to capacity
	for i := 0; i < 5; i++ {
		assert.NoError(t, buffer.Push(validIngot()))
	}

	// Verify backpressure
	err := buffer.Push(validIngot())
	assert.ErrorIs(t, err, ErrBufferFull)

	// Pop one to make space
	ctx := context.Background()
	_, err = buffer.Pop(ctx)
	assert.NoError(t, err)
	assert.Equal(t, 4, buffer.Len())

	// Should be able to push again
	err = buffer.Push(validIngot())
	assert.NoError(t, err)
	assert.Equal(t, 5, buffer.Len())
}
