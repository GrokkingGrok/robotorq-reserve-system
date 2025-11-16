package mint

import (
	"context"
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestNewIngotHashQueue(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))

	t.Run("valid creation", func(t *testing.T) {
		q, err := NewIngotHashQueue(2000, 1000, logger)
		require.NoError(t, err)
		assert.NotNil(t, q)
		assert.Equal(t, 2000, q.capacity)
		assert.Equal(t, 1000, q.batchSize)
		assert.Equal(t, 0, q.Len())
	})

	t.Run("invalid capacity", func(t *testing.T) {
		q, err := NewIngotHashQueue(0, 1000, logger)
		assert.Error(t, err)
		assert.Nil(t, q)
		assert.Contains(t, err.Error(), "capacity must be > 0")
	})

	t.Run("invalid batch size", func(t *testing.T) {
		q, err := NewIngotHashQueue(2000, 0, logger)
		assert.Error(t, err)
		assert.Nil(t, q)
		assert.Contains(t, err.Error(), "batchSize must be > 0")
	})

	t.Run("batch size exceeds capacity", func(t *testing.T) {
		q, err := NewIngotHashQueue(1000, 2000, logger)
		assert.Error(t, err)
		assert.Nil(t, q)
		assert.Contains(t, err.Error(), "batchSize must be > 0 and <= capacity")
	})
}

func TestIngotHashQueue_AddIngotHash(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	ctx := context.Background()

	t.Run("add single entry", func(t *testing.T) {
		q, _ := NewIngotHashQueue(100, 10, logger)

		entry := &models.IngotHashEntry{
			BranchHash:  "abc123" + string(make([]byte, 58)), // 64 chars
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			Timestamp:   time.Now(),
		}

		err := q.AddIngotHash(ctx, entry)
		assert.NoError(t, err)
		assert.Equal(t, 1, q.Len())
	})

	t.Run("add multiple entries", func(t *testing.T) {
		q, _ := NewIngotHashQueue(100, 10, logger)

		for i := 0; i < 25; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			err := q.AddIngotHash(ctx, entry)
			assert.NoError(t, err)
		}

		assert.Equal(t, 25, q.Len())
	})

	t.Run("context cancellation", func(t *testing.T) {
		q, _ := NewIngotHashQueue(10, 5, logger)

		// Fill queue to capacity
		for i := 0; i < 10; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(context.Background(), entry)
		}

		// Try to add with cancelled context (should fail immediately)
		ctx, cancel := context.WithCancel(context.Background())
		cancel()

		entry := &models.IngotHashEntry{
			BranchHash:  string(make([]byte, 64)),
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			Timestamp:   time.Now(),
		}

		err := q.AddIngotHash(ctx, entry)
		assert.Error(t, err)
		assert.Equal(t, context.Canceled, err)
	})
}

func TestIngotHashQueue_GetIngotHashes(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	ctx := context.Background()

	t.Run("get batch when ready", func(t *testing.T) {
		q, _ := NewIngotHashQueue(2000, 10, logger) // Batch size: 10

		// Add exactly 10 entries
		for i := 0; i < 10; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(ctx, entry)
		}

		// Should return immediately with 10 entries
		batch, err := q.GetIngotHashes(ctx)
		assert.NoError(t, err)
		assert.Equal(t, 10, len(batch))
		assert.Equal(t, 0, q.Len()) // Queue should be empty
	})

	t.Run("get multiple batches", func(t *testing.T) {
		q, _ := NewIngotHashQueue(2000, 10, logger)

		// Add 25 entries (2.5 batches)
		for i := 0; i < 25; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(ctx, entry)
		}

		// Get first batch
		batch1, err := q.GetIngotHashes(ctx)
		assert.NoError(t, err)
		assert.Equal(t, 10, len(batch1))
		assert.Equal(t, 15, q.Len())

		// Get second batch
		batch2, err := q.GetIngotHashes(ctx)
		assert.NoError(t, err)
		assert.Equal(t, 10, len(batch2))
		assert.Equal(t, 5, q.Len()) // 5 entries remain
	})

	t.Run("blocking until batch ready", func(t *testing.T) {
		q, _ := NewIngotHashQueue(2000, 10, logger)

		// Add 5 entries (not enough for batch)
		for i := 0; i < 5; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(ctx, entry)
		}

		retrieved := false
		var batch []*models.IngotHashEntry

		// Start GetIngotHashes in background (should block)
		go func() {
			batch, _ = q.GetIngotHashes(context.Background())
			retrieved = true
		}()

		// Verify it's blocking
		time.Sleep(100 * time.Millisecond)
		assert.False(t, retrieved, "GetIngotHashes should still be blocked")

		// Add 5 more entries to complete batch
		for i := 0; i < 5; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(ctx, entry)
		}

		// Wait for retrieval
		time.Sleep(100 * time.Millisecond)
		assert.True(t, retrieved, "GetIngotHashes should have unblocked")
		assert.Equal(t, 10, len(batch))
	})

	t.Run("context cancellation returns remaining", func(t *testing.T) {
		q, _ := NewIngotHashQueue(2000, 10, logger)

		// Add 7 entries (incomplete batch)
		for i := 0; i < 7; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(context.Background(), entry)
		}

		// GetIngotHashes with cancelled context
		ctx, cancel := context.WithCancel(context.Background())
		cancel()

		batch, err := q.GetIngotHashes(ctx)
		assert.NoError(t, err) // Should return remaining entries, not error
		assert.Equal(t, 7, len(batch))
		assert.Equal(t, 0, q.Len())
	})
}

func TestIngotHashQueue_ConcurrentAddGet(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	q, _ := NewIngotHashQueue(2000, 100, logger)

	var wg sync.WaitGroup

	// Producer: Add 1000 entries (10 batches)
	wg.Add(1)
	go func() {
		defer wg.Done()
		for i := 0; i < 1000; i++ {
			entry := &models.IngotHashEntry{
				BranchHash:  string(make([]byte, 64)),
				ContractIDs: []string{"contract-1"},
				DiggerIDs:   []string{"digger-1"},
				Timestamp:   time.Now(),
			}
			q.AddIngotHash(context.Background(), entry)
		}
	}()

	// Consumer: Retrieve 10 batches
	batchesRetrieved := 0
	wg.Add(1)
	go func() {
		defer wg.Done()
		for i := 0; i < 10; i++ {
			batch, err := q.GetIngotHashes(context.Background())
			assert.NoError(t, err)
			assert.Equal(t, 100, len(batch))
			batchesRetrieved++
		}
	}()

	wg.Wait()

	assert.Equal(t, 10, batchesRetrieved)
	assert.Equal(t, 0, q.Len())
}

func TestIngotHashQueue_Drain(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	q, _ := NewIngotHashQueue(2000, 10, logger)

	// Add 7 entries (incomplete batch)
	for i := 0; i < 7; i++ {
		entry := &models.IngotHashEntry{
			BranchHash:  string(make([]byte, 64)),
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			Timestamp:   time.Now(),
		}
		q.AddIngotHash(context.Background(), entry)
	}

	drained := q.Drain()
	assert.Equal(t, 7, len(drained))
	assert.Equal(t, 0, q.Len())

	// Drain again (should return empty)
	drained2 := q.Drain()
	assert.Equal(t, 0, len(drained2))
}

func TestIngotHashQueue_Len(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	q, _ := NewIngotHashQueue(2000, 10, logger)

	assert.Equal(t, 0, q.Len())

	entry := &models.IngotHashEntry{
		BranchHash:  string(make([]byte, 64)),
		ContractIDs: []string{"contract-1"},
		DiggerIDs:   []string{"digger-1"},
		Timestamp:   time.Now(),
	}
	q.AddIngotHash(context.Background(), entry)

	assert.Equal(t, 1, q.Len())
}
