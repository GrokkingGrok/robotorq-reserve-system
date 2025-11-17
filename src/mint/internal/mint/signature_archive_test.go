package mint

import (
	"fmt"
	"sync"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewSignatureArchive tests archive creation
func TestNewSignatureArchive(t *testing.T) {
	archive := NewSignatureArchive()

	assert.NotNil(t, archive)
	assert.Equal(t, 0, archive.Size())
}

// TestSignatureArchive_Store tests storing signatures
func TestSignatureArchive_Store(t *testing.T) {
	archive := NewSignatureArchive()

	record := &SignatureRecord{
		UnitID:     "RT-20251117-001",
		Signature:  "abc123",
		PublicKey:  "def456",
		MerkleRoot: "aabbccdd",
		MintedAt:   "2025-11-17T12:00:00Z",
		SignedAt:   "2025-11-17T12:00:01Z",
	}

	err := archive.Store(record)
	require.NoError(t, err)
	assert.Equal(t, 1, archive.Size())
	assert.True(t, archive.Has("RT-20251117-001"))
}

// TestSignatureArchive_Store_ValidationErrors tests validation
func TestSignatureArchive_Store_ValidationErrors(t *testing.T) {
	archive := NewSignatureArchive()

	tests := []struct {
		name        string
		record      *SignatureRecord
		expectedErr string
	}{
		{
			name: "missing unit_id",
			record: &SignatureRecord{
				Signature: "abc123",
				PublicKey: "def456",
			},
			expectedErr: "unit_id is required",
		},
		{
			name: "missing signature",
			record: &SignatureRecord{
				UnitID:    "RT-001",
				PublicKey: "def456",
			},
			expectedErr: "signature is required",
		},
		{
			name: "missing public_key",
			record: &SignatureRecord{
				UnitID:    "RT-001",
				Signature: "abc123",
			},
			expectedErr: "public_key is required",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := archive.Store(tt.record)
			require.Error(t, err)
			assert.Contains(t, err.Error(), tt.expectedErr)
		})
	}
}

// TestSignatureArchive_Get tests retrieving signatures
func TestSignatureArchive_Get(t *testing.T) {
	archive := NewSignatureArchive()

	record := &SignatureRecord{
		UnitID:     "RT-20251117-001",
		Signature:  "abc123",
		PublicKey:  "def456",
		MerkleRoot: "aabbccdd",
		MintedAt:   "2025-11-17T12:00:00Z",
		SignedAt:   "2025-11-17T12:00:01Z",
	}

	err := archive.Store(record)
	require.NoError(t, err)

	retrieved, err := archive.Get("RT-20251117-001")
	require.NoError(t, err)
	assert.Equal(t, "RT-20251117-001", retrieved.UnitID)
	assert.Equal(t, "abc123", retrieved.Signature)
	assert.Equal(t, "def456", retrieved.PublicKey)
}

// TestSignatureArchive_Get_NotFound tests missing signature
func TestSignatureArchive_Get_NotFound(t *testing.T) {
	archive := NewSignatureArchive()

	_, err := archive.Get("RT-nonexistent")
	require.Error(t, err)
	assert.Contains(t, err.Error(), "signature not found")
}

// TestSignatureArchive_ConcurrentAccess tests thread safety
func TestSignatureArchive_ConcurrentAccess(t *testing.T) {
	archive := NewSignatureArchive()

	var wg sync.WaitGroup
	numGoroutines := 100

	// Concurrent writes
	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			record := &SignatureRecord{
				UnitID:    fmt.Sprintf("RT-%d", id),
				Signature: fmt.Sprintf("sig-%d", id),
				PublicKey: "pubkey",
			}
			_ = archive.Store(record)
		}(i)
	}

	wg.Wait()

	// All signatures should be stored
	assert.Equal(t, numGoroutines, archive.Size())

	// Concurrent reads
	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			unitID := fmt.Sprintf("RT-%d", id)
			record, err := archive.Get(unitID)
			assert.NoError(t, err)
			assert.Equal(t, unitID, record.UnitID)
		}(i)
	}

	wg.Wait()
}
