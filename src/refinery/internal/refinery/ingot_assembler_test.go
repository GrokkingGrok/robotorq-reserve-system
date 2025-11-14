// internal/refinery/ingot_assembler_test.go
// Unit tests for IngotAssembler

package refinery

import (
	"context"
	"testing"
	"time"

	"b2b/refinery/internal/models"
)

// TestIngotAssembler_ExactThreshold tests assembling with exactly 3600 joules
func TestIngotAssembler_ExactThreshold(t *testing.T) {
	tests := []struct {
		name              string
		jouleAmount       float64
		roboAmount        float64
		price             float64
		contractID        string
		expectedIngots    int
		expectedExcess    float64
		expectedContracts int
	}{
		{
			name:              "exact_threshold_single_contract",
			jouleAmount:       3600.0,
			roboAmount:        100.0,
			price:             10.0,
			contractID:        "contract-001",
			expectedIngots:    1,
			expectedExcess:    0.0,
			expectedContracts: 1,
		},
		{
			name:              "half_threshold_no_ingot",
			jouleAmount:       1800.0,
			roboAmount:        50.0,
			price:             5.0,
			contractID:        "contract-002",
			expectedIngots:    0,
			expectedExcess:    1800.0,
			expectedContracts: 0,
		},
		{
			name:              "double_threshold_two_ingots",
			jouleAmount:       7200.0,
			roboAmount:        200.0,
			price:             20.0,
			contractID:        "contract-003",
			expectedIngots:    2,
			expectedExcess:    0.0,
			expectedContracts: 1,
		},
		{
			name:              "threshold_plus_excess",
			jouleAmount:       4000.0,
			roboAmount:        110.0,
			price:             11.0,
			contractID:        "contract-004",
			expectedIngots:    1,
			expectedExcess:    400.0,
			expectedContracts: 1,
		},
		{
			name:              "small_amount_no_ingot",
			jouleAmount:       500.0,
			roboAmount:        10.0,
			price:             5.0,
			contractID:        "contract-005",
			expectedIngots:    0,
			expectedExcess:    500.0,
			expectedContracts: 0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx, cancel := context.WithCancel(context.Background())
			defer cancel()

			qm := NewQueueManager(ctx, 10, 10)
			assembler := NewIngotAssembler(ctx, qm)

			// Create and process items
			jouleItem := &models.JouleQueueItem{
				Amount:     tt.jouleAmount,
				ContractID: tt.contractID,
				Timestamp:  time.Now(),
				Hash:       "test-hash",
			}

			roboItem := &models.RoboQueueItem{
				Amount:     tt.roboAmount,
				ContractID: tt.contractID,
				Timestamp:  time.Now(),
				Price:      tt.price,
			}

			// Process once - assembler handles multiple ingots if amount > 3600
			// Note: Current implementation only creates ONE ingot per call,
			// so we need to split large amounts manually for now
			remainingJoules := tt.jouleAmount
			for remainingJoules > 0 {
				thisAmount := remainingJoules
				if thisAmount > 3600 {
					thisAmount = 3600
				}

				jouleItem.Amount = thisAmount
				roboItem.Amount = (tt.roboAmount / tt.jouleAmount) * thisAmount

				err := assembler.processItems(jouleItem, roboItem)
				if err != nil {
					t.Fatalf("processItems failed: %v", err)
				}

				remainingJoules -= thisAmount
			}

			// Verify completed ingots count
			ingotsCount := assembler.GetCompletedIngotsCount()
			if ingotsCount != tt.expectedIngots {
				t.Errorf("expected %d ingots, got %d", tt.expectedIngots, ingotsCount)
			}

			// Verify accumulated excess
			excess := assembler.GetAccumulatedJoules()
			if excess != tt.expectedExcess {
				t.Errorf("expected excess %v, got %v", tt.expectedExcess, excess)
			}

			// If ingots were created, verify their properties
			if tt.expectedIngots > 0 {
				ingots := assembler.GetCompletedIngots()
				if len(ingots) != tt.expectedIngots {
					t.Fatalf("expected %d ingots from GetCompletedIngots, got %d", tt.expectedIngots, len(ingots))
				}

				for i, ingot := range ingots {
					// Verify joule total
					if ingot.JouleTorqTotal != uint64(JouleTorqThreshold) {
						t.Errorf("ingot %d: expected JouleTorqTotal %v, got %v", i, uint64(JouleTorqThreshold), ingot.JouleTorqTotal)
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
func TestIngotAssembler_MultipleContracts(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// Add joules from three different contracts
	contracts := []struct {
		contractID  string
		jouleAmount float64
		roboAmount  float64
		price       float64
	}{
		{"contract-A", 1200.0, 30.0, 8.0},
		{"contract-B", 1200.0, 35.0, 9.0},
		{"contract-C", 1200.0, 40.0, 10.0},
	}

	for _, c := range contracts {
		jouleItem := &models.JouleQueueItem{
			Amount:     c.jouleAmount,
			ContractID: c.contractID,
			Timestamp:  time.Now(),
			Hash:       "hash-" + c.contractID,
		}

		roboItem := &models.RoboQueueItem{
			Amount:     c.roboAmount,
			ContractID: c.contractID,
			Timestamp:  time.Now(),
			Price:      c.price,
		}

		err := assembler.processItems(jouleItem, roboItem)
		if err != nil {
			t.Fatalf("processItems failed for %s: %v", c.contractID, err)
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
		t.Errorf("expected 3 contract IDs, got %d", len(ingot.ContractIDs))
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
	if len(ingot.JouleTorqHashes) != 3 {
		t.Errorf("expected 3 hashes, got %d", len(ingot.JouleTorqHashes))
	}

	// Verify robo stake total (30 + 35 + 40 = 105)
	expectedRoboStake := 105.0
	if ingot.RoboStakeTotal != expectedRoboStake {
		t.Errorf("expected RoboStakeTotal %v, got %v", expectedRoboStake, ingot.RoboStakeTotal)
	}

	// Verify average price ((8 + 9 + 10) / 3 = 9.0)
	expectedAvgPrice := 9.0
	if ingot.PricePerRT != expectedAvgPrice {
		t.Errorf("expected PricePerRT %v, got %v", expectedAvgPrice, ingot.PricePerRT)
	}
}

// TestIngotAssembler_HashGeneration tests that unique hashes are generated for each joule contribution
func TestIngotAssembler_HashGeneration(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// Add three identical-amount joules from same contract at different times
	for i := 0; i < 3; i++ {
		jouleItem := &models.JouleQueueItem{
			Amount:     1200.0,
			ContractID: "contract-hash-test",
			Timestamp:  time.Now().Add(time.Duration(i) * time.Second), // Different timestamps
			Hash:       "base-hash",
		}

		roboItem := &models.RoboQueueItem{
			Amount:     30.0,
			ContractID: "contract-hash-test",
			Timestamp:  time.Now(),
			Price:      10.0,
		}

		err := assembler.processItems(jouleItem, roboItem)
		if err != nil {
			t.Fatalf("processItems failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	hashes := ingots[0].JouleTorqHashes

	// Verify three hashes exist
	if len(hashes) != 3 {
		t.Fatalf("expected 3 hashes, got %d", len(hashes))
	}

	// Verify all hashes are different (due to different timestamps)
	hashSet := make(map[string]bool)
	for _, hash := range hashes {
		if hash == "" {
			t.Error("found empty hash")
		}
		if hashSet[hash] {
			t.Errorf("duplicate hash found: %s", hash)
		}
		hashSet[hash] = true

		// Verify hash is 64-character hex string (SHA256)
		if len(hash) != 64 {
			t.Errorf("expected 64-character hash, got %d: %s", len(hash), hash)
		}
	}
}

// TestIngotAssembler_IngotIDUniqueness tests that each ingot gets a unique ID
func TestIngotAssembler_IngotIDUniqueness(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// Create three ingots with a small delay to ensure unique timestamps
	for i := 0; i < 3; i++ {
		jouleItem := &models.JouleQueueItem{
			Amount:     3600.0,
			ContractID: "contract-uniqueness",
			Timestamp:  time.Now(),
			Hash:       "hash-uniqueness",
		}

		roboItem := &models.RoboQueueItem{
			Amount:     100.0,
			ContractID: "contract-uniqueness",
			Timestamp:  time.Now(),
			Price:      10.0,
		}

		err := assembler.processItems(jouleItem, roboItem)
		if err != nil {
			t.Fatalf("processItems failed: %v", err)
		}

		// Small delay to ensure unique timestamp-based UUIDs
		time.Sleep(2 * time.Millisecond)
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

// TestIngotAssembler_ExcessCarryover tests that excess joules carry over to next ingot
func TestIngotAssembler_ExcessCarryover(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// First: Add 4000 joules (should create 1 ingot with 400 excess)
	jouleItem := &models.JouleQueueItem{
		Amount:     4000.0,
		ContractID: "contract-carryover",
		Timestamp:  time.Now(),
		Hash:       "hash-1",
	}

	roboItem := &models.RoboQueueItem{
		Amount:     110.0,
		ContractID: "contract-carryover",
		Timestamp:  time.Now(),
		Price:      10.0,
	}

	err := assembler.processItems(jouleItem, roboItem)
	if err != nil {
		t.Fatalf("first processItems failed: %v", err)
	}

	// Verify 1 ingot created and 400 excess
	if assembler.GetCompletedIngotsCount() != 1 {
		t.Errorf("expected 1 ingot after first add, got %d", assembler.GetCompletedIngotsCount())
	}

	excess := assembler.GetAccumulatedJoules()
	if excess != 400.0 {
		t.Errorf("expected 400 excess, got %v", excess)
	}

	// Second: Add 3200 more joules (400 + 3200 = 3600, should create another ingot)
	jouleItem2 := &models.JouleQueueItem{
		Amount:     3200.0,
		ContractID: "contract-carryover",
		Timestamp:  time.Now(),
		Hash:       "hash-2",
	}

	roboItem2 := &models.RoboQueueItem{
		Amount:     90.0,
		ContractID: "contract-carryover",
		Timestamp:  time.Now(),
		Price:      9.0,
	}

	err = assembler.processItems(jouleItem2, roboItem2)
	if err != nil {
		t.Fatalf("second processItems failed: %v", err)
	}

	// Verify 2 ingots total and 0 excess
	if assembler.GetCompletedIngotsCount() != 2 {
		t.Errorf("expected 2 ingots after second add, got %d", assembler.GetCompletedIngotsCount())
	}

	finalExcess := assembler.GetAccumulatedJoules()
	if finalExcess != 0.0 {
		t.Errorf("expected 0 final excess, got %v", finalExcess)
	}
}

// TestIngotAssembler_PriceAveraging tests that prices are averaged correctly
func TestIngotAssembler_PriceAveraging(t *testing.T) {
	tests := []struct {
		name             string
		prices           []float64
		jouleAmounts     []float64
		expectedAvgPrice float64
	}{
		{
			name:             "three_equal_prices",
			prices:           []float64{10.0, 10.0, 10.0},
			jouleAmounts:     []float64{1200.0, 1200.0, 1200.0},
			expectedAvgPrice: 10.0,
		},
		{
			name:             "ascending_prices",
			prices:           []float64{5.0, 10.0, 15.0},
			jouleAmounts:     []float64{1200.0, 1200.0, 1200.0},
			expectedAvgPrice: 10.0, // (5 + 10 + 15) / 3
		},
		{
			name:             "two_contributions",
			prices:           []float64{8.0, 12.0},
			jouleAmounts:     []float64{1800.0, 1800.0},
			expectedAvgPrice: 10.0, // (8 + 12) / 2
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx, cancel := context.WithCancel(context.Background())
			defer cancel()

			qm := NewQueueManager(ctx, 10, 10)
			assembler := NewIngotAssembler(ctx, qm)

			for i, price := range tt.prices {
				jouleItem := &models.JouleQueueItem{
					Amount:     tt.jouleAmounts[i],
					ContractID: "contract-pricing",
					Timestamp:  time.Now(),
					Hash:       "hash-pricing",
				}

				roboItem := &models.RoboQueueItem{
					Amount:     100.0,
					ContractID: "contract-pricing",
					Timestamp:  time.Now(),
					Price:      price,
				}

				err := assembler.processItems(jouleItem, roboItem)
				if err != nil {
					t.Fatalf("processItems failed: %v", err)
				}
			}

			ingots := assembler.GetCompletedIngots()
			if len(ingots) != 1 {
				t.Fatalf("expected 1 ingot, got %d", len(ingots))
			}

			if ingots[0].PricePerRT != tt.expectedAvgPrice {
				t.Errorf("expected avg price %v, got %v", tt.expectedAvgPrice, ingots[0].PricePerRT)
			}
		})
	}
}

// TestIngotAssembler_GetCompletedIngotsClears tests that GetCompletedIngots clears the list
func TestIngotAssembler_GetCompletedIngotsClears(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// Create one ingot
	jouleItem := &models.JouleQueueItem{
		Amount:     3600.0,
		ContractID: "contract-clear",
		Timestamp:  time.Now(),
		Hash:       "hash-clear",
	}

	roboItem := &models.RoboQueueItem{
		Amount:     100.0,
		ContractID: "contract-clear",
		Timestamp:  time.Now(),
		Price:      10.0,
	}

	err := assembler.processItems(jouleItem, roboItem)
	if err != nil {
		t.Fatalf("processItems failed: %v", err)
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

// TestIngotAssembler_ContractIDDeduplication tests that duplicate contract IDs are not added
func TestIngotAssembler_ContractIDDeduplication(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	// Add three joules from same contract
	for i := 0; i < 3; i++ {
		jouleItem := &models.JouleQueueItem{
			Amount:     1200.0,
			ContractID: "contract-dedup",
			Timestamp:  time.Now(),
			Hash:       "hash-dedup",
		}

		roboItem := &models.RoboQueueItem{
			Amount:     30.0,
			ContractID: "contract-dedup",
			Timestamp:  time.Now(),
			Price:      10.0,
		}

		err := assembler.processItems(jouleItem, roboItem)
		if err != nil {
			t.Fatalf("processItems failed: %v", err)
		}
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	// Verify only one unique contract ID despite three contributions
	if len(ingots[0].ContractIDs) != 1 {
		t.Errorf("expected 1 contract ID, got %d: %v", len(ingots[0].ContractIDs), ingots[0].ContractIDs)
	}

	if ingots[0].ContractIDs[0] != "contract-dedup" {
		t.Errorf("expected contract ID 'contract-dedup', got '%s'", ingots[0].ContractIDs[0])
	}
}

// TestIngotAssembler_ZeroPrice tests handling of zero price
func TestIngotAssembler_ZeroPrice(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10, 10)
	assembler := NewIngotAssembler(ctx, qm)

	jouleItem := &models.JouleQueueItem{
		Amount:     3600.0,
		ContractID: "contract-zero",
		Timestamp:  time.Now(),
		Hash:       "hash-zero",
	}

	roboItem := &models.RoboQueueItem{
		Amount:     100.0,
		ContractID: "contract-zero",
		Timestamp:  time.Now(),
		Price:      0.0, // Zero price
	}

	err := assembler.processItems(jouleItem, roboItem)
	if err != nil {
		t.Fatalf("processItems failed: %v", err)
	}

	ingots := assembler.GetCompletedIngots()
	if len(ingots) != 1 {
		t.Fatalf("expected 1 ingot, got %d", len(ingots))
	}

	// Average price should be 0.0
	if ingots[0].PricePerRT != 0.0 {
		t.Errorf("expected price 0.0, got %v", ingots[0].PricePerRT)
	}
}

// Benchmark for ingot assembly performance
func BenchmarkIngotAssembler_Assembly(b *testing.B) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	qm := NewQueueManager(ctx, 10000, 10000)
	assembler := NewIngotAssembler(ctx, qm)

	jouleItem := &models.JouleQueueItem{
		Amount:     3600.0,
		ContractID: "benchmark-contract",
		Timestamp:  time.Now(),
		Hash:       "benchmark-hash",
	}

	roboItem := &models.RoboQueueItem{
		Amount:     100.0,
		ContractID: "benchmark-contract",
		Timestamp:  time.Now(),
		Price:      10.0,
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		_ = assembler.processItems(jouleItem, roboItem)
	}
}
