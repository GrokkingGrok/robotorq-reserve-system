package refinery

import (
	"context"
	"sync"
	"testing"
	"time"

	"b2b/refinery/internal/models"
)

func TestQueueManager_AddJoule(t *testing.T) {
	tests := []struct {
		name     string
		capacity int
		items    int
		wantErr  bool
		errType  error
	}{
		{
			name:     "add single item",
			capacity: 10,
			items:    1,
			wantErr:  false,
		},
		{
			name:     "add multiple items within capacity",
			capacity: 10,
			items:    5,
			wantErr:  false,
		},
		{
			name:     "fill to capacity",
			capacity: 10,
			items:    10,
			wantErr:  false,
		},
		{
			name:     "exceed capacity",
			capacity: 5,
			items:    6,
			wantErr:  true,
			errType:  models.ErrQueueFull,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := context.Background()
			qm := NewQueueManager(ctx, tt.capacity, 10)

			var lastErr error
			for i := 0; i < tt.items; i++ {
				item := &models.JouleQueueItem{
					Amount:     float64(i + 1),
					ContractID: "test-contract",
					Hash:       "test-hash",
				}
				err := qm.AddJoule(item)
				if err != nil {
					lastErr = err
				}
			}

			if tt.wantErr && lastErr == nil {
				t.Errorf("Expected error, got nil")
			}
			if !tt.wantErr && lastErr != nil {
				t.Errorf("Unexpected error: %v", lastErr)
			}
			if tt.wantErr && lastErr != tt.errType {
				t.Errorf("Expected error %v, got %v", tt.errType, lastErr)
			}

			// Verify queue size
			expectedSize := tt.items
			if tt.wantErr {
				expectedSize = tt.capacity // Should be at capacity
			}
			if qm.GetJouleQueueSize() != expectedSize {
				t.Errorf("Queue size = %d, want %d", qm.GetJouleQueueSize(), expectedSize)
			}
		})
	}
}

func TestQueueManager_AddRobo(t *testing.T) {
	tests := []struct {
		name     string
		capacity int
		items    int
		wantErr  bool
		errType  error
	}{
		{
			name:     "add single item",
			capacity: 10,
			items:    1,
			wantErr:  false,
		},
		{
			name:     "add multiple items within capacity",
			capacity: 10,
			items:    5,
			wantErr:  false,
		},
		{
			name:     "fill to capacity",
			capacity: 10,
			items:    10,
			wantErr:  false,
		},
		{
			name:     "exceed capacity",
			capacity: 5,
			items:    6,
			wantErr:  true,
			errType:  models.ErrQueueFull,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := context.Background()
			qm := NewQueueManager(ctx, 10, tt.capacity)

			var lastErr error
			for i := 0; i < tt.items; i++ {
				item := &models.RoboQueueItem{
					Amount:     float64(i + 1),
					Price:      10.0,
					ContractID: "test-contract",
				}
				err := qm.AddRobo(item)
				if err != nil {
					lastErr = err
				}
			}

			if tt.wantErr && lastErr == nil {
				t.Errorf("Expected error, got nil")
			}
			if !tt.wantErr && lastErr != nil {
				t.Errorf("Unexpected error: %v", lastErr)
			}
			if tt.wantErr && lastErr != tt.errType {
				t.Errorf("Expected error %v, got %v", tt.errType, lastErr)
			}

			// Verify queue size
			expectedSize := tt.items
			if tt.wantErr {
				expectedSize = tt.capacity // Should be at capacity
			}
			if qm.GetRoboQueueSize() != expectedSize {
				t.Errorf("Queue size = %d, want %d", qm.GetRoboQueueSize(), expectedSize)
			}
		})
	}
}

func TestQueueManager_GetJoule(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 10, 10)

	// Add items to queue
	expectedAmounts := []float64{100.0, 200.0, 300.0}
	for i, amt := range expectedAmounts {
		item := &models.JouleQueueItem{
			Amount:     amt,
			ContractID: "contract-" + string(rune(i)),
			Hash:       "hash-" + string(rune(i)),
		}
		if err := qm.AddJoule(item); err != nil {
			t.Fatalf("Failed to add item: %v", err)
		}
	}

	// Retrieve items and verify FIFO order
	for i, expectedAmt := range expectedAmounts {
		item, err := qm.GetJoule()
		if err != nil {
			t.Errorf("GetJoule() error = %v", err)
			continue
		}
		if item.Amount != expectedAmt {
			t.Errorf("Item %d: amount = %v, want %v", i, item.Amount, expectedAmt)
		}
	}

	// Queue should be empty now
	if qm.GetJouleQueueSize() != 0 {
		t.Errorf("Queue size after draining = %d, want 0", qm.GetJouleQueueSize())
	}
}

func TestQueueManager_GetRobo(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 10, 10)

	// Add items to queue
	expectedAmounts := []float64{50.0, 75.0, 100.0}
	for i, amt := range expectedAmounts {
		item := &models.RoboQueueItem{
			Amount:     amt,
			Price:      10.0 + float64(i),
			ContractID: "contract-" + string(rune(i)),
		}
		if err := qm.AddRobo(item); err != nil {
			t.Fatalf("Failed to add item: %v", err)
		}
	}

	// Retrieve items and verify FIFO order
	for i, expectedAmt := range expectedAmounts {
		item, err := qm.GetRobo()
		if err != nil {
			t.Errorf("GetRobo() error = %v", err)
			continue
		}
		if item.Amount != expectedAmt {
			t.Errorf("Item %d: amount = %v, want %v", i, item.Amount, expectedAmt)
		}
	}

	// Queue should be empty now
	if qm.GetRoboQueueSize() != 0 {
		t.Errorf("Queue size after draining = %d, want 0", qm.GetRoboQueueSize())
	}
}

func TestQueueManager_ContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	qm := NewQueueManager(ctx, 10, 10)

	// Add an item
	item := &models.JouleQueueItem{
		Amount:     100.0,
		ContractID: "test-contract",
		Hash:       "test-hash",
	}
	if err := qm.AddJoule(item); err != nil {
		t.Fatalf("Failed to add item: %v", err)
	}

	// Cancel context
	cancel()

	// Wait a moment for cancellation to propagate
	time.Sleep(10 * time.Millisecond)

	// Try to add another item - should fail with shutdown error
	item2 := &models.JouleQueueItem{
		Amount:     200.0,
		ContractID: "test-contract-2",
		Hash:       "test-hash-2",
	}
	err := qm.AddJoule(item2)
	if err != models.ErrQueueShuttingDown {
		t.Errorf("Expected ErrQueueShuttingDown, got %v", err)
	}
}

func TestQueueManager_ConcurrentAdds(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 1000, 1000)

	// Launch multiple goroutines adding items concurrently
	numGoroutines := 10
	itemsPerGoroutine := 50

	var wg sync.WaitGroup
	errors := make(chan error, numGoroutines*itemsPerGoroutine)

	// Concurrent joule adds
	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for j := 0; j < itemsPerGoroutine; j++ {
				item := &models.JouleQueueItem{
					Amount:     float64(id*itemsPerGoroutine + j),
					ContractID: "contract",
					Hash:       "hash",
				}
				if err := qm.AddJoule(item); err != nil {
					errors <- err
				}
			}
		}(i)
	}

	// Concurrent robo adds
	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for j := 0; j < itemsPerGoroutine; j++ {
				item := &models.RoboQueueItem{
					Amount:     float64(id*itemsPerGoroutine + j),
					Price:      10.0,
					ContractID: "contract",
				}
				if err := qm.AddRobo(item); err != nil {
					errors <- err
				}
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	// Check for errors
	for err := range errors {
		t.Errorf("Concurrent add failed: %v", err)
	}

	// Verify all items were added
	expectedCount := numGoroutines * itemsPerGoroutine
	if qm.GetJouleQueueSize() != expectedCount {
		t.Errorf("Joule queue size = %d, want %d", qm.GetJouleQueueSize(), expectedCount)
	}
	if qm.GetRoboQueueSize() != expectedCount {
		t.Errorf("Robo queue size = %d, want %d", qm.GetRoboQueueSize(), expectedCount)
	}
}

func TestQueueManager_ConcurrentAddAndGet(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 1000, 1000)

	itemsToProcess := 100
	var wg sync.WaitGroup

	// Producer goroutine
	wg.Add(1)
	go func() {
		defer wg.Done()
		for i := 0; i < itemsToProcess; i++ {
			item := &models.JouleQueueItem{
				Amount:     float64(i),
				ContractID: "contract",
				Hash:       "hash",
			}
			if err := qm.AddJoule(item); err != nil {
				t.Errorf("Failed to add item: %v", err)
			}
			time.Sleep(1 * time.Millisecond) // Small delay to allow consumer to process
		}
	}()

	// Consumer goroutine
	retrieved := make([]float64, 0, itemsToProcess)
	var mu sync.Mutex
	wg.Add(1)
	go func() {
		defer wg.Done()
		for i := 0; i < itemsToProcess; i++ {
			item, err := qm.GetJoule()
			if err != nil {
				t.Errorf("Failed to get item: %v", err)
				continue
			}
			mu.Lock()
			retrieved = append(retrieved, item.Amount)
			mu.Unlock()
		}
	}()

	wg.Wait()

	// Verify all items were retrieved
	if len(retrieved) != itemsToProcess {
		t.Errorf("Retrieved %d items, want %d", len(retrieved), itemsToProcess)
	}

	// Queue should be empty
	if qm.GetJouleQueueSize() != 0 {
		t.Errorf("Queue size = %d, want 0", qm.GetJouleQueueSize())
	}
}

func TestQueueManager_Capacity(t *testing.T) {
	ctx := context.Background()
	jouleCapacity := 100
	roboCapacity := 200

	qm := NewQueueManager(ctx, jouleCapacity, roboCapacity)

	// Test GetJouleCapacity
	if qm.GetJouleCapacity() != jouleCapacity {
		t.Errorf("GetJouleCapacity() = %d, want %d", qm.GetJouleCapacity(), jouleCapacity)
	}

	// Test GetRoboCapacity
	if qm.GetRoboCapacity() != roboCapacity {
		t.Errorf("GetRoboCapacity() = %d, want %d", qm.GetRoboCapacity(), roboCapacity)
	}
}

func TestQueueManager_SizeTracking(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 10, 10)

	// Initially empty
	if qm.GetJouleQueueSize() != 0 {
		t.Errorf("Initial joule queue size = %d, want 0", qm.GetJouleQueueSize())
	}
	if qm.GetRoboQueueSize() != 0 {
		t.Errorf("Initial robo queue size = %d, want 0", qm.GetRoboQueueSize())
	}

	// Add items and verify size increases
	for i := 1; i <= 5; i++ {
		jouleItem := &models.JouleQueueItem{
			Amount:     float64(i),
			ContractID: "contract",
			Hash:       "hash",
		}
		qm.AddJoule(jouleItem)

		roboItem := &models.RoboQueueItem{
			Amount:     float64(i),
			Price:      10.0,
			ContractID: "contract",
		}
		qm.AddRobo(roboItem)

		if qm.GetJouleQueueSize() != i {
			t.Errorf("After adding %d items, joule queue size = %d, want %d", i, qm.GetJouleQueueSize(), i)
		}
		if qm.GetRoboQueueSize() != i {
			t.Errorf("After adding %d items, robo queue size = %d, want %d", i, qm.GetRoboQueueSize(), i)
		}
	}

	// Remove items and verify size decreases
	for i := 4; i >= 0; i-- {
		qm.GetJoule()
		qm.GetRobo()

		if qm.GetJouleQueueSize() != i {
			t.Errorf("After removing item, joule queue size = %d, want %d", qm.GetJouleQueueSize(), i)
		}
		if qm.GetRoboQueueSize() != i {
			t.Errorf("After removing item, robo queue size = %d, want %d", qm.GetRoboQueueSize(), i)
		}
	}
}

func TestQueueManager_Close(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 10, 10)

	// Add some items
	jouleItem := &models.JouleQueueItem{
		Amount:     100.0,
		ContractID: "contract",
		Hash:       "hash",
	}
	qm.AddJoule(jouleItem)

	roboItem := &models.RoboQueueItem{
		Amount:     50.0,
		Price:      10.0,
		ContractID: "contract",
	}
	qm.AddRobo(roboItem)

	// Close the queue manager
	qm.Close()

	// Note: After closing, channels are closed, so we can't add more items
	// This test just verifies Close() doesn't panic
}

func TestQueueManager_BackpressureHandling(t *testing.T) {
	ctx := context.Background()
	capacity := 3
	qm := NewQueueManager(ctx, capacity, capacity)

	// Fill the queue to capacity
	for i := 0; i < capacity; i++ {
		item := &models.JouleQueueItem{
			Amount:     float64(i),
			ContractID: "contract",
			Hash:       "hash",
		}
		if err := qm.AddJoule(item); err != nil {
			t.Fatalf("Failed to add item %d: %v", i, err)
		}
	}

	// Try to add one more - should fail immediately (non-blocking)
	start := time.Now()
	item := &models.JouleQueueItem{
		Amount:     999.0,
		ContractID: "contract",
		Hash:       "hash",
	}
	err := qm.AddJoule(item)
	elapsed := time.Since(start)

	if err != models.ErrQueueFull {
		t.Errorf("Expected ErrQueueFull, got %v", err)
	}

	// Verify it didn't block (should be nearly instantaneous)
	if elapsed > 100*time.Millisecond {
		t.Errorf("AddJoule blocked for %v, expected immediate return", elapsed)
	}

	// Verify queue size is still at capacity
	if qm.GetJouleQueueSize() != capacity {
		t.Errorf("Queue size = %d, want %d", qm.GetJouleQueueSize(), capacity)
	}
}

func BenchmarkQueueManager_AddJoule(b *testing.B) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100000, 100000)

	item := &models.JouleQueueItem{
		Amount:     100.0,
		ContractID: "benchmark-contract",
		Hash:       "benchmark-hash",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		qm.AddJoule(item)
	}
}

func BenchmarkQueueManager_GetJoule(b *testing.B) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100000, 100000)

	// Pre-fill the queue
	for i := 0; i < b.N; i++ {
		item := &models.JouleQueueItem{
			Amount:     float64(i),
			ContractID: "benchmark-contract",
			Hash:       "benchmark-hash",
		}
		qm.AddJoule(item)
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		qm.GetJoule()
	}
}
