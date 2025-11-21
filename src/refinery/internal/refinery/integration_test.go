// internal/refinery/integration_test.go
// End-to-end integration test for Refinery pipeline (Phase 2):
// Hash Queue → Merkle Tree Assembly → NATS publishing

package refinery

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"testing"
	"time"

	"b2b/refinery/internal/config"

	"github.com/nats-io/nats.go"
)

// TestRefineryIntegration_EndToEnd tests the complete Phase 2 pipeline:
// 1. Add hashes to queue directly (simulating ore extraction)
// 2. Verify ingot assembly at 3600 hash threshold
// 3. Verify NATS publishing of Phase2Ingot
func TestRefineryIntegration_EndToEnd(t *testing.T) {
	// 1. Start embedded NATS server
	ns, natsURL := startTestNATSServer(t)
	defer ns.Shutdown()

	// 2. Set up context with timeout
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// 3. Initialize configuration
	cfg := &config.Config{
		NatsURL:            natsURL,
		IngotBatchInterval: 60 * time.Second,
		MintMaxRetries:     3,
		MintBaseDelay:      1 * time.Second,
		JouleQueueSize:     4000, // Large enough for 3600+ hashes
		RoboQueueSize:      1000,
	}

	// 4. Initialize components
	logger := slog.Default()
	queueManager := NewQueueManager(ctx, cfg.JouleQueueSize)
	ingotAssembler := NewPhase2IngotAssembler(ctx, queueManager, logger)
	mintClient, err := NewMintClient(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to create mint client: %v", err)
	}
	defer mintClient.Close()

	// 5. Start ingot assembler in background
	go ingotAssembler.Start()

	// 6. Subscribe to NATS topic to capture published ingots
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	publishedIngots := make(chan *Phase2Ingot, 10)
	sub, err := nc.Subscribe("mint.phase2.ingots", func(msg *nats.Msg) {
		var batch map[string]interface{}
		if err := json.Unmarshal(msg.Data, &batch); err != nil {
			t.Errorf("failed to unmarshal batch: %v", err)
			return
		}

		// Extract ingots from batch envelope
		ingotsRaw := batch["ingots"]
		ingotsJSON, _ := json.Marshal(ingotsRaw)

		var ingots []*Phase2Ingot
		if err := json.Unmarshal(ingotsJSON, &ingots); err != nil {
			t.Errorf("failed to unmarshal ingots: %v", err)
			return
		}

		for _, ingot := range ingots {
			publishedIngots <- ingot
		}
	})
	if err != nil {
		t.Fatalf("failed to subscribe to NATS: %v", err)
	}
	defer sub.Unsubscribe()

	// 7. Add exactly 3600 hashes to the queue
	// This should trigger assembly of exactly 1 ingot
	const hashesNeeded = 3600
	const contractID = "integration-test-contract"

	t.Logf("Adding %d hashes to queue...", hashesNeeded)

	for i := 0; i < hashesNeeded; i++ {
		hash := fmt.Sprintf("test-hash-%d", i)
		roboStake := 0.00416 // RoboStake per unit

		err := queueManager.AddHash(hash, contractID, "test-digger", roboStake)
		if err != nil {
			t.Fatalf("failed to add hash %d: %v", i, err)
		}

		if (i + 1) % 1000 == 0 {
			t.Logf("Added %d hashes", i+1)
		}
	}

	t.Log("All hashes added, waiting for ingot assembly...")
	time.Sleep(2 * time.Second)

	// 8. Get completed ingots from assembler
	completedIngots := ingotAssembler.GetCompletedIngots()

	if len(completedIngots) == 0 {
		t.Fatal("No ingots were assembled from the hashes")
	}

	t.Logf("Found %d assembled ingot(s), publishing to NATS...", len(completedIngots))

	// 9. Publish ingots to NATS using Phase2 method
	if err := mintClient.PublishPhase2Batch(completedIngots); err != nil {
		t.Fatalf("failed to publish Phase2 batch: %v", err)
	}

	// 10. Wait for NATS message
	select {
	case ingot := <-publishedIngots:
		t.Log("Received Phase2Ingot from NATS")

		// Validate ingot
		if ingot.ID == "" {
			t.Error("ingot ID is empty")
		}
		if ingot.BranchHash == "" {
			t.Error("branch hash is empty")
		}
		if ingot.HashCount != 3600 {
			t.Errorf("expected 3600 hashes, got %d", ingot.HashCount)
		}
		if len(ingot.ContractIDs) == 0 {
			t.Error("ingot has no contract IDs")
		}
		if ingot.RoboStakeTotal == 0 {
			t.Error("robo stake total is zero")
		}

		expectedRoboStake := 0.00416 * 3600 // 3600 hashes × 0.00416 per hash
		if ingot.RoboStakeTotal < expectedRoboStake-0.1 || ingot.RoboStakeTotal > expectedRoboStake+0.1 {
			t.Errorf("expected robo stake ~%.2f, got %.2f", expectedRoboStake, ingot.RoboStakeTotal)
		}

		t.Logf("Ingot validated: id=%s, branch_hash=%s, hash_count=%d, contracts=%d, robo_stake=%.2f",
			ingot.ID,
			ingot.BranchHash,
			ingot.HashCount,
			len(ingot.ContractIDs),
			ingot.RoboStakeTotal,
		)

		t.Log("✅ Integration test PASSED: Full Phase 2 pipeline working correctly")

	case <-time.After(5 * time.Second):
		t.Fatal("timeout waiting for NATS Phase2Ingot publication")
	}
}

// TestRefineryIntegration_MultipleIngots tests assembling multiple ingots from multiple hash batches
func TestRefineryIntegration_MultipleIngots(t *testing.T) {
	// Start embedded NATS server
	ns, natsURL := startTestNATSServer(t)
	defer ns.Shutdown()

	// Set up context
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Initialize configuration
	cfg := &config.Config{
		NatsURL:            natsURL,
		IngotBatchInterval: 60 * time.Second,
		MintMaxRetries:     3,
		MintBaseDelay:      1 * time.Second,
		JouleQueueSize:     8000, // Need space for 2 batches
		RoboQueueSize:      1000,
	}

	// Initialize components
	logger := slog.Default()
	queueManager := NewQueueManager(ctx, cfg.JouleQueueSize)
	ingotAssembler := NewPhase2IngotAssembler(ctx, queueManager, logger)
	mintClient, err := NewMintClient(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to create mint client: %v", err)
	}
	defer mintClient.Close()

	// Start ingot assembler
	go ingotAssembler.Start()

	// Subscribe to NATS
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	publishedBatches := make(chan map[string]interface{}, 10)
	sub, err := nc.Subscribe("mint.phase2.ingots", func(msg *nats.Msg) {
		var batch map[string]interface{}
		if err := json.Unmarshal(msg.Data, &batch); err != nil {
			t.Errorf("failed to unmarshal batch: %v", err)
			return
		}
		publishedBatches <- batch
	})
	if err != nil {
		t.Fatalf("failed to subscribe to NATS: %v", err)
	}
	defer sub.Unsubscribe()

	// Send 2 complete batches (7200 hashes total = 2 ingots)
	t.Log("Adding 7200 hashes for 2 ingots...")

	for i := 0; i < 7200; i++ {
		hash := fmt.Sprintf("test-hash-batch-%d", i)
		roboStake := 0.00416
		contract := fmt.Sprintf("contract-%d", i/3600) // 0 for first 3600, 1 for next 3600

		err := queueManager.AddHash(hash, contract, "test-digger", roboStake)
		if err != nil {
			t.Fatalf("failed to add hash %d: %v", i, err)
		}

		if (i + 1) % 3600 == 0 {
			t.Logf("Added %d hashes", i+1)
		}
	}

	t.Log("Waiting for ingots to be assembled...")
	time.Sleep(2 * time.Second)

	// Get completed ingots
	completedIngots := ingotAssembler.GetCompletedIngots()

	if len(completedIngots) < 2 {
		t.Fatalf("expected 2 ingots, got %d", len(completedIngots))
	}

	t.Logf("Found %d ingots, publishing to NATS...", len(completedIngots))

	// Publish first batch
	if err := mintClient.PublishPhase2Batch(completedIngots[:1]); err != nil {
		t.Fatalf("failed to publish first batch: %v", err)
	}

	// Wait for both published
	select {
	case batch := <-publishedBatches:
		count := int(batch["count"].(float64))
		if count != 1 {
			t.Errorf("expected 1 ingot in batch, got %d", count)
		}
		t.Log("✅ Multiple ingots test PASSED")

	case <-time.After(5 * time.Second):
		t.Fatal("timeout waiting for batch publication")
	}
}


// BatchEnvelope matches the structure in mint_client.go
type BatchEnvelope struct {
	BatchID   string                 `json:"batch_id"`
	Timestamp time.Time              `json:"timestamp"`
	Count     int                    `json:"count"`
	Ingots    []*Phase2Ingot         `json:"ingots"`
}
