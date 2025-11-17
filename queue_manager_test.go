package refinery

import (
"context"
"crypto/sha256"
"encoding/hex"
"fmt"
"testing"

"b2b/refinery/internal/models"
)

func generateTestHash(index int) string {
h := sha256.Sum256([]byte(fmt.Sprintf("test-hash-%d", index)))
return hex.EncodeToString(h[:])
}

func TestQueueManager_AddHash(t *testing.T) {
ctx := context.Background()
qm := NewQueueManager(ctx, 10)

hash := generateTestHash(0)
err := qm.AddHash(hash, "test-contract", "test-digger")
if err != nil {
t.Errorf("AddHash failed: %v", err)
}

if qm.GetQueueSize() != 1 {
t.Errorf("Queue size = %d, want 1", qm.GetQueueSize())
}
}

func TestQueueManager_Capacity(t *testing.T) {
ctx := context.Background()
capacity := 5
qm := NewQueueManager(ctx, capacity)

for i := 0; i < capacity; i++ {
hash := generateTestHash(i)
if err := qm.AddHash(hash, "test-contract", "test-digger"); err != nil {
t.Fatalf("AddHash %d failed: %v", i, err)
}
}

hash := generateTestHash(capacity)
err := qm.AddHash(hash, "test-contract", "test-digger")
if err != models.ErrQueueFull {
t.Errorf("Expected ErrQueueFull, got %v", err)
}
}