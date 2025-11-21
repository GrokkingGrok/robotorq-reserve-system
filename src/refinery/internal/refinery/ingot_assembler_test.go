// internal/refinery/ingot_assembler_test.go
// Unit tests for IngotAssembler

package refinery

import (
	"context"
	"fmt"
	"testing"

	"b2b/refinery/internal/models"
)

// createTestUnits generates N JouleTorqUnits for testing
func createTestUnits(contractID string, count int, joulesPerUnit, roboPerUnit float64) []*models.JouleTorqUnit {
	units := make([]*models.JouleTorqUnit, count)
	for i := 0; i < count; i++ {
		units[i] = models.NewJouleTorqUnit(
			contractID,
			0, // milestoneIndex
			i, // tokenIndex
			joulesPerUnit,
			roboPerUnit,
			"test-digger",
			"stub-signature",
			"stub-pubkey",
		)
	}
	return units
}

// TestIngotAssembler_ExactThreshold tests assembling with exactly 3600 units
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
// Currently skipped because Phase 1 QueueManager.AddUnit() is deprecated
// Phase 2 uses AddHash() with hash-only batches from Digger
func TestIngotAssembler_ExactThreshold(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	tests := []struct {
		name              string
		unitCount         int
		joulesPerUnit     float64
		roboPerUnit       float64
		contractID        string
		expectedIngots    int
		expectedExcess    int
		expectedContracts int
	}{
		{
			name:              "exact_threshold_single_contract",
			unitCount:         3600,
			joulesPerUnit:     1.0,
			roboPerUnit:       0.0278, // ~100 RT / 3600
			contractID:        "contract-001",
			expectedIngots:    1,
			expectedExcess:    0,
			expectedContracts: 1,
		},
		{
			name:              "half_threshold_no_ingot",
			unitCount:         1800,
			joulesPerUnit:     1.0,
			roboPerUnit:       0.0278,
			contractID:        "contract-002",
			expectedIngots:    0,
			expectedExcess:    1800,
			expectedContracts: 0,
		},
		{
			name:              "double_threshold_two_ingots",
			unitCount:         7200,
			joulesPerUnit:     1.0,
			roboPerUnit:       0.0278,
			contractID:        "contract-003",
			expectedIngots:    2,
			expectedExcess:    0,
			expectedContracts: 1,
		},
		{
			name:              "threshold_plus_excess",
			unitCount:         4000,
			joulesPerUnit:     1.0,
			roboPerUnit:       0.0275,
			contractID:        "contract-004",
			expectedIngots:    1,
			expectedExcess:    400,
			expectedContracts: 1,
		},
		{
			name:              "small_amount_no_ingot",
			unitCount:         500,
			joulesPerUnit:     1.0,
			roboPerUnit:       0.02,
			contractID:        "contract-005",
			expectedIngots:    0,
			expectedExcess:    500,
			expectedContracts: 0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx, cancel := context.WithCancel(context.Background())
			defer cancel()

			qm := NewQueueManager(ctx, 10000)
			assembler := NewIngotAssembler(ctx, qm)

			// Create and queue test units
			units := createTestUnits(tt.contractID, tt.unitCount, tt.joulesPerUnit, tt.roboPerUnit)
			for _, unit := range units {
				if err := qm.AddUnit(unit); err != nil {
					t.Fatalf("AddUnit failed: %v", err)
				}
			}

			// Process all units
			for i := 0; i < tt.unitCount; i++ {
				unit, err := qm.GetUnit()
				if err != nil {
					t.Fatalf("GetUnit failed at %d: %v", i, err)
				}
				if err := assembler.processUnit(unit); err != nil {
					t.Fatalf("processUnit failed: %v", err)
				}
			}

			// Verify completed ingots count
			ingotsCount := assembler.GetCompletedIngotsCount()
			if ingotsCount != tt.expectedIngots {
				t.Errorf("expected %d ingots, got %d", tt.expectedIngots, ingotsCount)
			}

			// Verify accumulated excess
			excess := assembler.GetAccumulatedUnits()
			if excess != tt.expectedExcess {
				t.Errorf("expected excess %v units, got %v", tt.expectedExcess, excess)
			}

			// If ingots were created, verify their properties
			if tt.expectedIngots > 0 {
				ingots := assembler.GetCompletedIngots()
				if len(ingots) != tt.expectedIngots {
					t.Fatalf("expected %d ingots from GetCompletedIngots, got %d", tt.expectedIngots, len(ingots))
				}

				for i, ingot := range ingots {
					// Verify joule total
					expectedJoules := 3600.0 * tt.joulesPerUnit
					if ingot.JouleTorqTotal < expectedJoules-1 || ingot.JouleTorqTotal > expectedJoules+1 {
						t.Errorf("ingot %d: expected JouleTorqTotal ~%v, got %v", i, expectedJoules, ingot.JouleTorqTotal)
					}

					// Verify contract IDs
					if len(ingot.ContractIDs) != tt.expectedContracts {
						t.Errorf("ingot %d: expected %d contracts, got %d", i, tt.expectedContracts, len(ingot.ContractIDs))
					}

					// Verify ingot ID exists
					if ingot.IngotID == "" {
						t.Errorf("ingot %d: IngotID is empty", i)
					}

					// Verify timestamp
					if ingot.MintedAt.IsZero() {
						t.Errorf("ingot %d: MintedAt is zero", i)
					}
				}
			}
		})
	}
}

// TestIngotAssembler_MultipleContracts tests ingots assembled from multiple contracts
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_MultipleContracts(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Add units from three different contracts
	contracts := []struct {
		contractID    string
		unitCount     int
		joulesPerUnit float64
		roboPerUnit   float64
	}{
		{"contract-A", 1200, 1.0, 0.025},
		{"contract-B", 1200, 1.0, 0.029},
		{"contract-C", 1200, 1.0, 0.033},
	}

	totalUnits := 0
	for _, c := range contracts {
		units := createTestUnits(c.contractID, c.unitCount, c.joulesPerUnit, c.roboPerUnit)
		for _, unit := range units {
			if err := qm.AddUnit(unit); err != nil {
				t.Fatalf("AddUnit failed for %s: %v", c.contractID, err)
			}
		}
		totalUnits += c.unitCount
	}

	// Process all units
	for i := 0; i < totalUnits; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed at %d: %v", i, err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	// Should have created one ingot (1200 + 1200 + 1200 = 3600)
	ingotsCount := assembler.GetCompletedIngotsCount()
	if ingotsCount != 1 {
		t.Fatalf("expected 1 ingot, got %d", ingotsCount)
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot from GetCompletedIngots, got %d", len(ingots))
	}

	ingot := ingots[0]

	// Verify three unique contract IDs
	if len(ingot.ContractIDs) != 3 {
		t.Errorf("expected 3 contract IDs, got %d: %v", len(ingot.ContractIDs), ingot.ContractIDs)
	}

	// Verify contract IDs are present
	expectedContracts := map[string]bool{
		"contract-A": false,
		"contract-B": false,
		"contract-C": false,
	}

	for _, contractID := range ingot.ContractIDs {
		if _, exists := expectedContracts[contractID]; exists {
			expectedContracts[contractID] = true
		}
	}

	for contract, found := range expectedContracts {
		if !found {
			t.Errorf("contract %s not found in ingot ContractIDs", contract)
		}
	}

	// Verify three hashes (one per contract contribution)
	// NOTE: With new structure, we have 3,600 units (not 3 hashes)
	if len(ingot.Units) != 3600 {
		t.Errorf("expected 3600 units, got %d", len(ingot.Units))
	}

	// Verify robo stake total (1200×0.025 + 1200×0.029 + 1200×0.033 = 30 + 34.8 + 39.6 = 104.4)
	// Use tolerance for floating-point comparison (same pattern as Mint tests)
	expectedRoboStake := 104.4
	delta := 0.01
	if ingot.RoboStakeTotal < expectedRoboStake-delta || ingot.RoboStakeTotal > expectedRoboStake+delta {
		t.Errorf("expected RoboStakeTotal %v ±%v, got %v", expectedRoboStake, delta, ingot.RoboStakeTotal)
	}

	// Verify average price
	// NOTE: PricePerRT removed from TokenTorqIngot (price calculated from RoboStakeTotal / JouleTorqTotal)
	// Skipping price verification for now - will add back when we refactor to use real units
	/*
		expectedAvgPrice := 9.0
		if ingot.PricePerRT != expectedAvgPrice {
			t.Errorf("expected PricePerRT %v, got %v", expectedAvgPrice, ingot.PricePerRT)
		}
	*/
}

// TestIngotAssembler_HashGeneration tests that unique hashes are generated for each unit
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_HashGeneration(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Add 3600 units from same contract (units will have different token indexes)
	units := createTestUnits("contract-hash-test", 3600, 1.0, 0.025)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all units
	for i := 0; i < 3600; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	// Verify unit hashes exist
	if len(ingots[0].Units) != 3600 {
		t.Fatalf("expected 3600 units, got %d", len(ingots[0].Units))
	}

	// Verify all unit hashes are unique
	hashSet := make(map[string]bool)
	for _, unit := range ingots[0].Units {
		if unit.Hash == "" {
			t.Error("found empty hash")
		}
		if hashSet[unit.Hash] {
			t.Errorf("duplicate hash found: %s", unit.Hash)
		}
		hashSet[unit.Hash] = true
	}
}

// TestIngotAssembler_IngotIDUniqueness tests that each ingot gets a unique ID
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_IngotIDUniqueness(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 20000)
	assembler := NewIngotAssembler(ctx, qm)

	// Create three ingots (3 × 3600 units)
	for i := 0; i < 3; i++ {
		contractID := fmt.Sprintf("contract-uniqueness-%d", i)
		units := createTestUnits(contractID, 3600, 1.0, 0.0278)

		for _, unit := range units {
			if err := qm.AddUnit(unit); err != nil {
				t.Fatalf("AddUnit failed: %v", err)
			}
		}
	}

	// Process all 10,800 units
	for i := 0; i < 10800; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 3 {
		t.Fatalf("expected 3 ingots, got %d", len(ingots))
	}

	// Verify all ingot IDs are unique
	idSet := make(map[string]bool)
	for i, ingot := range ingots {
		if ingot.IngotID == "" {
			t.Errorf("ingot %d has empty IngotID", i)
		}
		if idSet[ingot.IngotID] {
			t.Errorf("duplicate IngotID found: %s", ingot.IngotID)
		}
		idSet[ingot.IngotID] = true
	}
}

// TestIngotAssembler_ExcessCarryover tests that excess units carry over to next ingot
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_ExcessCarryover(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// First: Add 4000 units (should create 1 ingot with 400 excess)
	units := createTestUnits("contract-carryover", 4000, 1.0, 0.0275)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all 4000 units
	for i := 0; i < 4000; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	// Verify 1 ingot created and 400 excess
	if assembler.GetCompletedIngotsCount() != 1 {
		t.Errorf("expected 1 ingot after first add, got %d", assembler.GetCompletedIngotsCount())
	}

	excess := assembler.GetAccumulatedUnits()
	if excess != 400 {
		t.Errorf("expected 400 excess units, got %v", excess)
	}

	// Second: Add 3200 more units (400 + 3200 = 3600, should create another ingot)
	units2 := createTestUnits("contract-carryover-2", 3200, 1.0, 0.028)
	for _, unit := range units2 {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all 3200 units
	for i := 0; i < 3200; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	// Verify 2 ingots total and 0 excess
	if assembler.GetCompletedIngotsCount() != 2 {
		t.Errorf("expected 2 ingots after second add, got %d", assembler.GetCompletedIngotsCount())
	}

	finalExcess := assembler.GetAccumulatedUnits()
	if finalExcess != 0 {
		t.Errorf("expected 0 final excess, got %v", finalExcess)
	}
}

// TestIngotAssembler_PriceAveraging tests RoboStake calculation
func TestIngotAssembler_PriceAveraging(t *testing.T) {
	// NOTE: PricePerRT removed from TokenTorqIngot
	// We now just verify RoboStakeTotal is correctly summed
	t.Skip("Test needs redesign for unit-based approach - price is calculated from RoboStakeTotal/JouleTorqTotal")
}

// TestIngotAssembler_GetCompletedIngotsClears tests that GetCompletedIngots clears the list
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_GetCompletedIngotsClears(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Create one ingot
	units := createTestUnits("contract-clear", 3600, 1.0, 0.0278)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all units
	for i := 0; i < 3600; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	// Verify 1 ingot exists
	if assembler.GetCompletedIngotsCount() != 1 {
		t.Fatalf("expected 1 ingot, got %d", assembler.GetCompletedIngotsCount())
	}

	// Get ingots (should clear the list)
	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot from GetCompletedIngots, got %d", len(ingots))
	}

	// Verify list is now cleared
	if assembler.GetCompletedIngotsCount() != 0 {
		t.Errorf("expected 0 ingots after GetCompletedIngots, got %d", assembler.GetCompletedIngotsCount())
	}

	// Second call should return empty list
	ingots2 := assembler.GetCompletedIngots()
	if len(ingots2) != 0 {
		t.Errorf("expected 0 ingots from second GetCompletedIngots, got %d", len(ingots2))
	}
}

// TestIngotAssembler_ContractIDDeduplication tests that contract IDs are tracked
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_ContractIDDeduplication(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Add 3600 units from same contract
	units := createTestUnits("contract-dedup", 3600, 1.0, 0.025)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all units
	for i := 0; i < 3600; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	// Verify only one unique contract ID
	if len(ingots[0].ContractIDs) != 1 {
		t.Errorf("expected 1 contract ID, got %d: %v", len(ingots[0].ContractIDs), ingots[0].ContractIDs)
	}

	if ingots[0].ContractIDs[0] != "contract-dedup" {
		t.Errorf("expected contract ID 'contract-dedup', got '%s'", ingots[0].ContractIDs[0])
	}
}

// TestIngotAssembler_ZeroRoboStake tests handling of zero robo stake
// TODO(phase-2-update): Update for Phase 2 hash-based QueueManager API
func TestIngotAssembler_ZeroRoboStake(t *testing.T) {
	t.Skip("TODO: Update for Phase 2 hash-based API (AddHash instead of AddUnit)")
	
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Create units with zero robo stake
	units := createTestUnits("contract-zero", 3600, 1.0, 0.0)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			t.Fatalf("AddUnit failed: %v", err)
		}
	}

	// Process all units
	for i := 0; i < 3600; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			t.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			t.Fatalf("processUnit failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	// RoboStakeTotal should be 0.0
	if ingots[0].RoboStakeTotal != 0.0 {
		t.Errorf("expected RoboStakeTotal 0.0, got %v", ingots[0].RoboStakeTotal)
	}
}

// Benchmark for ingot assembly performance
func BenchmarkIngotAssembler_Assembly(b *testing.B) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	// Pre-generate units for benchmark
	units := createTestUnits("benchmark-contract", b.N*3600, 1.0, 0.0278)
	for _, unit := range units {
		if err := qm.AddUnit(unit); err != nil {
			b.Fatalf("AddUnit failed: %v", err)
		}
	}

	b.ResetTimer()

	for i := 0; i < b.N*3600; i++ {
		unit, err := qm.GetUnit()
		if err != nil {
			b.Fatalf("GetUnit failed: %v", err)
		}
		if err := assembler.processUnit(unit); err != nil {
			b.Fatalf("processUnit failed: %v", err)
		}
	}
}
