package refinery

import (
	"context"
	"sync"
	"testing"
	"time"
)

// Phase 2: Hash-based queue tests

func TestQueueManager_AddHash(t *testing.T) {
	tests := []struct {
		name     string
		capacity int
		items    int
		wantErr  bool
	}{
		{
			name:     "add single hash",
			capacity: 10,
			items:    1,
			wantErr:  false,
		},
		{
			name:     "add multiple hashes within capacity",
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
			name:     "exceed capacity (backpressure)",
			capacity: 5,
			items:    6,
			wantErr:  true, // Returns ErrQueueFull for backpressure
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := context.Background()
			qm := NewQueueManager(ctx, tt.capacity)

			var lastErr error
			for i := 0; i < tt.items; i++ {
				hash := "hash-" + string(rune(i))
				err := qm.AddHash(hash, "test-contract", "test-digger", 0.001)
				if err != nil {
					lastErr = err
				}
			}

			if tt.wantErr && lastErr == nil {
				t.Errorf("Expected backpressure error, got nil")
			}
			if !tt.wantErr && lastErr != nil {
				t.Errorf("Unexpected error: %v", lastErr)
			}

			// Verify queue size
			expectedSize := tt.items
			if tt.wantErr {
				expectedSize = tt.capacity // Should be at capacity
			}
			if qm.GetQueueSize() != expectedSize {
				t.Errorf("Queue size = %d, want %d", qm.GetQueueSize(), expectedSize)
			}
		})
	}
}

func TestQueueManager_GetHashes(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100)

	// Add hashes to queue
	expectedHashes := []string{"hash-1", "hash-2", "hash-3", "hash-4", "hash-5"}
	for _, hash := range expectedHashes {
		if err := qm.AddHash(hash, "contract-A", "digger-1", 0.001); err != nil {
			t.Fatalf("Failed to add hash: %v", err)
		}
	}

	// Retrieve 3 hashes
	retrieved, err := qm.GetHashes(3)
	if err != nil {
		t.Errorf("GetHashes() error = %v", err)
	}

	if len(retrieved) != 3 {
		t.Errorf("Retrieved %d hashes, want 3", len(retrieved))
	}

	// Verify FIFO order and content
	for i := 0; i < 3; i++ {
		if retrieved[i].Hash != expectedHashes[i] {
			t.Errorf("Hash %d: got %s, want %s", i, retrieved[i].Hash, expectedHashes[i])
		}
		if retrieved[i].ContractID != "contract-A" {
			t.Errorf("Hash %d: contract = %s, want contract-A", i, retrieved[i].ContractID)
		}
	}

	// Queue should have 2 hashes remaining
	if qm.GetQueueSize() != 2 {
		t.Errorf("Queue size after retrieval = %d, want 2", qm.GetQueueSize())
	}
}

func TestQueueManager_GetHashesBlocking(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100)

	// Test blocking behavior: GetHashes should block until N hashes available
	done := make(chan bool, 1)
	start := time.Now()

	go func() {
		// This should block until 5 hashes are available
		hashes, err := qm.GetHashes(5)
		if err != nil {
			t.Errorf("GetHashes error: %v", err)
		}
		if len(hashes) != 5 {
			t.Errorf("Got %d hashes, want 5", len(hashes))
		}
		done <- true
	}()

	// Give goroutine time to start blocking
	time.Sleep(100 * time.Millisecond)

	// Add 3 hashes - should not unblock yet
	for i := 0; i < 3; i++ {
		qm.AddHash("hash-"+string(rune(i)), "contract", "digger", 0.001)
	}

	select {
	case <-done:
		t.Errorf("Unblocked too early, still waiting for 5 hashes")
	case <-time.After(50 * time.Millisecond):
		// Expected: should still be blocked
	}

	// Add remaining 2 hashes - should unblock
	for i := 3; i < 5; i++ {
		qm.AddHash("hash-"+string(rune(i)), "contract", "digger", 0.001)
	}

	select {
	case <-done:
		elapsed := time.Since(start)
		// Should have unblocked relatively quickly (within 200ms)
		if elapsed > 500*time.Millisecond {
			t.Errorf("Unblocked too slowly: %v", elapsed)
		}
	case <-time.After(500 * time.Millisecond):
		t.Errorf("Timeout waiting for GetHashes to unblock")
	}
}

func TestQueueManager_ConcurrentAdds(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 1000)

	// Launch multiple goroutines adding hashes concurrently
	numGoroutines := 10
	hashesPerGoroutine := 50

	var wg sync.WaitGroup
	errors := make(chan error, numGoroutines*hashesPerGoroutine)

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for j := 0; j < hashesPerGoroutine; j++ {
				hash := "hash-" + string(rune(id*100+j))
				if err := qm.AddHash(hash, "contract", "digger", 0.001); err != nil {
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

	// Verify all hashes were added
	expectedCount := numGoroutines * hashesPerGoroutine
	if qm.GetQueueSize() != expectedCount {
		t.Errorf("Queue size = %d, want %d", qm.GetQueueSize(), expectedCount)
	}
}

func TestQueueManager_Capacity(t *testing.T) {
	ctx := context.Background()
	capacity := 100
	qm := NewQueueManager(ctx, capacity)

	// Test capacity
	if qm.GetCapacity() != capacity {
		t.Errorf("GetCapacity() = %d, want %d", qm.GetCapacity(), capacity)
	}
}

func TestQueueManager_Close(t *testing.T) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 10)

	// Add some hashes
	qm.AddHash("hash-1", "contract", "digger", 0.001)

	// Close the queue manager
	qm.Close()

	// Note: After closing, we can't add more hashes
	// This test just verifies Close() doesn't panic
}

func BenchmarkQueueManager_AddHash(b *testing.B) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100000)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		qm.AddHash("benchmark-hash", "contract", "digger", 0.001)
	}
}

func BenchmarkQueueManager_GetHashes(b *testing.B) {
	ctx := context.Background()
	qm := NewQueueManager(ctx, 100000)

	// Pre-fill the queue with enough hashes
	for i := 0; i < b.N*10; i++ {
		qm.AddHash("hash-"+string(rune(i)), "contract", "digger", 0.001)
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		qm.GetHashes(10)
	}
}
