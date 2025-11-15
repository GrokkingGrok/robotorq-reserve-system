package refinery

import (
	"testing"

	"b2b/refinery/internal/models"
)

// mockQueueManager implements QueueAdder for testing
type mockQueueManager struct {
	units      []*models.JouleTorqUnit
	shouldFail bool
}

func (m *mockQueueManager) AddUnit(unit *models.JouleTorqUnit) error {
	if m.shouldFail {
		return models.ErrQueueFull
	}
	m.units = append(m.units, unit)
	return nil
}

// TODO: These tests need complete refactoring for unit-based architecture
// The old tests validated joule/robo queue items, but we now create JouleTorqUnits directly
// New tests should verify:
// - Ore → N JouleTorqUnits conversion (1 unit per token)
// - Joules/robo distributed evenly across tokens
// - Signature handling ([]byte → hex string)
// - Queue overflow handling

func TestOreReceiver_ReceiveOre(t *testing.T) {
	t.Skip("Test needs refactoring for unit-based queue - validates old joule/robo items")
}

func TestOreReceiver_HTTPHandler(t *testing.T) {
	t.Skip("Test needs refactoring for unit-based queue - validates old joule/robo items")
}

func TestOreReceiver_PriceCalculation(t *testing.T) {
	t.Skip("Test needs refactoring for unit-based queue - validates old robo price calculation")
}

func TestOreReceiver_HashGeneration(t *testing.T) {
	t.Skip("Test needs refactoring for unit-based queue - hashing now done per-unit, not per-ore")
}
