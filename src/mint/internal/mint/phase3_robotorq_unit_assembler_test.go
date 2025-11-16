package mint

import (
	"context"
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestNewPhase3RoboTorqUnitAssembler tests assembler creation
func TestNewPhase3RoboTorqUnitAssembler(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	assert.NotNil(t, assembler)
	assert.Equal(t, 10, assembler.channelCapacity)
	assert.NotNil(t, assembler.unitChannel)
	assert.NotNil(t, assembler.logger)
	assert.NotNil(t, assembler.metrics)
}

// TestNewPhase3RoboTorqUnitAssembler_DefaultCapacity tests default channel capacity
func TestNewPhase3RoboTorqUnitAssembler_DefaultCapacity(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	// Test with 0 capacity (should default to 10)
	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 0)
	assert.Equal(t, 10, assembler.channelCapacity)

	// Test with negative capacity (should default to 10)
	assembler2 := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, -5)
	assert.Equal(t, 10, assembler2.channelCapacity)
}

// TestPhase3RoboTorqUnitAssembler_AssembleUnit tests unit assembly
func TestPhase3RoboTorqUnitAssembler_AssembleUnit(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	reg := prometheus.NewRegistry()
	metrics := NewPhase3AssemblerMetrics(reg)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	// Create test merkle result
	merkleResult := &Level2MerkleResult{
		MerkleRoot:  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		TreeHeight:  10,
		ContractIDs: []string{"contract-1", "contract-2", "contract-3"},
		DiggerIDs:   []string{"digger-1", "digger-2"},
		RefineryIDs: []string{"refinery-1"},
	}

	unit, err := assembler.assembleUnit(merkleResult)

	require.NoError(t, err)
	assert.NotNil(t, unit)
	assert.NotEmpty(t, unit.UnitID)
	assert.Equal(t, merkleResult.MerkleRoot, unit.MerkleRoot)
	assert.False(t, unit.MintedAt.IsZero())

	// Verify size is minimal (pure hash-only)
	assert.Less(t, unit.SizeBytes(), 200, "Unit should be <200 bytes")

	t.Logf("Assembled unit: %s, size: %d bytes", unit.UnitID, unit.SizeBytes())
}

// TestPhase3RoboTorqUnitAssembler_AssembleUnit_NilResult tests nil merkle result
func TestPhase3RoboTorqUnitAssembler_AssembleUnit_NilResult(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	unit, err := assembler.assembleUnit(nil)

	assert.Error(t, err)
	assert.Nil(t, unit)
	assert.Contains(t, err.Error(), "merkle result is nil")
}

// TestPhase3RoboTorqUnitAssembler_AssembleUnit_EmptyMerkleRoot tests empty merkle root
func TestPhase3RoboTorqUnitAssembler_AssembleUnit_EmptyMerkleRoot(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	merkleResult := &Level2MerkleResult{
		MerkleRoot:  "", // Empty
		TreeHeight:  10,
		ContractIDs: []string{"contract-1"},
		DiggerIDs:   []string{"digger-1"},
		RefineryIDs: []string{"refinery-1"},
	}

	unit, err := assembler.assembleUnit(merkleResult)

	assert.Error(t, err)
	assert.Nil(t, unit)
	assert.Contains(t, err.Error(), "merkle root is empty")
}

// TestPhase3RoboTorqUnitAssembler_MetricsRecorded tests metrics recording
func TestPhase3RoboTorqUnitAssembler_MetricsRecorded(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	reg := prometheus.NewRegistry()
	metrics := NewPhase3AssemblerMetrics(reg)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	// Create test merkle result
	merkleResult := &Level2MerkleResult{
		MerkleRoot:  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
		TreeHeight:  10,
		ContractIDs: []string{"c1", "c2", "c3", "c4", "c5"},
		DiggerIDs:   []string{"d1", "d2", "d3"},
		RefineryIDs: []string{"r1", "r2"},
	}

	// Assemble unit
	_, err = assembler.assembleUnit(merkleResult)
	require.NoError(t, err)

	// Gather metrics
	metricFamilies, err := reg.Gather()
	require.NoError(t, err)

	// Verify metrics exist
	metricNames := make(map[string]bool)
	for _, mf := range metricFamilies {
		metricNames[mf.GetName()] = true
	}

	assert.True(t, metricNames["mint_phase3_units_assembled_total"])
	assert.True(t, metricNames["mint_phase3_assembly_duration_seconds"])
	assert.True(t, metricNames["mint_phase3_contracts_per_unit"])
	assert.True(t, metricNames["mint_phase3_diggers_per_unit"])
	assert.True(t, metricNames["mint_phase3_refineries_per_unit"])

	t.Logf("Metrics recorded: %d metric families", len(metricFamilies))
}

// TestPhase3RoboTorqUnitAssembler_Start tests assembler lifecycle
func TestPhase3RoboTorqUnitAssembler_Start(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start assembler in background
	go assembler.Start(ctx)

	// Give it time to start
	time.Sleep(100 * time.Millisecond)

	// Add 1000 ingot hashes to trigger merkle build
	for i := 0; i < 1000; i++ {
		entry := &models.IngotHashEntry{
			BranchHash:  generateTestHash(i),
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			RefineryID:  "refinery-1",
			Timestamp:   time.Now().UTC(),
		}
		err := queue.AddIngotHash(ctx, entry)
		require.NoError(t, err)
	}

	// Wait for unit to be assembled
	select {
	case unit := <-assembler.GetUnitChannel():
		assert.NotNil(t, unit)
		assert.NotEmpty(t, unit.UnitID)
		assert.NotEmpty(t, unit.MerkleRoot)
		assert.Less(t, unit.SizeBytes(), 200, "Unit should be <200 bytes")
		t.Logf("Received Phase3 unit: %s, size: %d bytes", unit.UnitID, unit.SizeBytes())

	case <-time.After(5 * time.Second):
		t.Fatal("Timeout waiting for Phase3 unit")
	}

	// Cancel context
	cancel()

	// Verify channel closes
	select {
	case _, ok := <-assembler.GetUnitChannel():
		assert.False(t, ok, "Channel should be closed")
	case <-time.After(1 * time.Second):
		t.Fatal("Channel did not close after context cancellation")
	}
}

// TestPhase3RoboTorqUnitAssembler_GracefulShutdown tests graceful shutdown
func TestPhase3RoboTorqUnitAssembler_GracefulShutdown(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(2000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	ctx, cancel := context.WithCancel(context.Background())

	// Start assembler
	go assembler.Start(ctx)

	// Give it time to start
	time.Sleep(100 * time.Millisecond)

	// Cancel immediately (no data sent)
	cancel()

	// Verify channel closes within reasonable time
	select {
	case _, ok := <-assembler.GetUnitChannel():
		assert.False(t, ok, "Channel should be closed on graceful shutdown")
	case <-time.After(1 * time.Second):
		t.Fatal("Channel did not close during graceful shutdown")
	}
}

// TestPhase3RoboTorqUnitAssembler_MultipleUnits tests assembling multiple units
func TestPhase3RoboTorqUnitAssembler_MultipleUnits(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(4000, 1000, logger) // Capacity for 4 batches
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 10)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start assembler
	go assembler.Start(ctx)

	// Add 3000 ingot hashes (should produce 3 units)
	for i := 0; i < 3000; i++ {
		entry := &models.IngotHashEntry{
			BranchHash:  generateTestHash(i),
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			RefineryID:  "refinery-1",
			Timestamp:   time.Now().UTC(),
		}
		err := queue.AddIngotHash(ctx, entry)
		require.NoError(t, err)
	}

	// Receive 3 units
	receivedUnits := 0
	for receivedUnits < 3 {
		select {
		case unit := <-assembler.GetUnitChannel():
			assert.NotNil(t, unit)
			receivedUnits++
			t.Logf("Received unit %d: %s", receivedUnits, unit.UnitID)

		case <-time.After(10 * time.Second):
			t.Fatalf("Timeout waiting for unit %d (received %d)", receivedUnits+1, receivedUnits)
		}
	}

	assert.Equal(t, 3, receivedUnits, "Should receive 3 units from 3000 ingots")
}

// TestPhase3RoboTorqUnitAssembler_ChannelBackpressure tests channel backpressure handling
func TestPhase3RoboTorqUnitAssembler_ChannelBackpressure(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
	metrics := NewPhase3AssemblerMetrics(nil)
	queue, err := NewIngotHashQueue(4000, 1000, logger)
	require.NoError(t, err)
	merkleBuilder := NewLevel2MerkleBuilder(queue, logger)

	// Small channel capacity to test backpressure
	assembler := NewPhase3RoboTorqUnitAssembler(logger, metrics, merkleBuilder, 2)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start assembler
	go assembler.Start(ctx)

	// Add 2000 ingot hashes (should produce 2 units)
	for i := 0; i < 2000; i++ {
		entry := &models.IngotHashEntry{
			BranchHash:  generateTestHash(i),
			ContractIDs: []string{"contract-1"},
			DiggerIDs:   []string{"digger-1"},
			RefineryID:  "refinery-1",
			Timestamp:   time.Now().UTC(),
		}
		err := queue.AddIngotHash(ctx, entry)
		require.NoError(t, err)
	}

	// Wait for channel to fill (2 units)
	time.Sleep(500 * time.Millisecond)

	// Channel should have 2 units (at capacity)
	receivedUnits := 0
	for receivedUnits < 2 {
		select {
		case unit := <-assembler.GetUnitChannel():
			assert.NotNil(t, unit)
			receivedUnits++
			t.Logf("Drained unit %d: %s", receivedUnits, unit.UnitID)

		case <-time.After(5 * time.Second):
			t.Fatalf("Timeout draining channel (got %d units)", receivedUnits)
		}
	}

	assert.Equal(t, 2, receivedUnits, "Should receive 2 units")
}
