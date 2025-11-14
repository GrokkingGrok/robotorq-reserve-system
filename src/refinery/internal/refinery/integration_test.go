// internal/refinery/integration_test.go
// End-to-end integration test for Refinery pipeline:
// HTTP ore reception → Queue → Ingot Assembly → NATS publishing

package refinery

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"b2b/refinery/internal/config"
	"b2b/refinery/internal/models"

	"github.com/nats-io/nats.go"
)

// TestRefineryIntegration_EndToEnd tests the complete pipeline:
// 1. Send JouleTorqOre via HTTP
// 2. Verify queuing
// 3. Verify ingot assembly at threshold
// 4. Verify NATS publishing with batch envelope
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
		JouleQueueSize:     1000,
		RoboQueueSize:      1000,
	}

	// 4. Initialize components
	queueManager := NewQueueManager(ctx, cfg.JouleQueueSize, cfg.RoboQueueSize)
	ingotAssembler := NewIngotAssembler(ctx, queueManager)
	mintClient, err := NewMintClient(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to create mint client: %v", err)
	}
	defer mintClient.Close()

	oreReceiver := NewOreReceiver(queueManager)

	// 5. Start ingot assembler in background
	go ingotAssembler.Start()

	// 6. Subscribe to NATS topic to capture published batches
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	publishedBatches := make(chan *BatchEnvelope, 10)
	sub, err := nc.Subscribe("mint.ingots", func(msg *nats.Msg) {
		var envelope BatchEnvelope
		if err := json.Unmarshal(msg.Data, &envelope); err != nil {
			t.Errorf("failed to unmarshal batch envelope: %v", err)
			return
		}
		publishedBatches <- &envelope
	})
	if err != nil {
		t.Fatalf("failed to subscribe to NATS: %v", err)
	}
	defer sub.Unsubscribe()

	// 6. Create HTTP test server for ore receiver
	handler := http.HandlerFunc(oreReceiver.HTTPHandler)
	server := httptest.NewServer(handler)
	defer server.Close()

	// 7. Send enough ores to trigger ingot assembly
	// Threshold is 3600 joules = 1 ingot
	// We'll send 4 ores of 900 joules each = 3600 total
	const joulePerOre = 900.0
	const oresNeeded = 4

	for i := 0; i < oresNeeded; i++ {
		ore := &models.JouleTorqOre{
			DiggerID:        fmt.Sprintf("digger-test-%d", i),
			ContractID:      "integration-test-contract",
			TokensGenerated: 60,
			Joules:          uint64(joulePerOre),
			MilestoneIndex:  uint32(i),
			Timestamp:       uint64(time.Now().Unix()),
			RoboStakeAmount: 0.00416,
		}

		// Send ore via HTTP POST
		oreJSON, _ := json.Marshal(ore)
		resp, err := http.Post(server.URL, "application/json", bytes.NewBuffer(oreJSON))
		if err != nil {
			t.Fatalf("failed to POST ore %d: %v", i, err)
		}

		if resp.StatusCode != http.StatusOK {
			body, _ := io.ReadAll(resp.Body)
			t.Fatalf("ore %d rejected: %d - %s", i, resp.StatusCode, string(body))
		}

		var response map[string]interface{}
		if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
			t.Fatalf("failed to decode response for ore %d: %v", i, err)
		}
		resp.Body.Close()

		// Verify response
		if response["status"] != "accepted" {
			t.Errorf("ore %d: expected status 'accepted', got '%v'", i, response["status"])
		}
		if response["contract_id"] != ore.ContractID {
			t.Errorf("ore %d: expected contract_id '%s', got '%v'", i, ore.ContractID, response["contract_id"])
		}

		t.Logf("Ore %d sent successfully: %d joules", i, ore.Joules)
	}

	// 8. Wait for ingot to be assembled
	t.Log("Waiting for ingot assembly...")
	time.Sleep(2 * time.Second)

	// 9. Manually trigger batch publishing (since we don't have BatchSender running)
	ingotAssembler.mu.Lock()
	ingotsToPublish := make([]*models.TokenTorqIngot, len(ingotAssembler.completedIngots))
	copy(ingotsToPublish, ingotAssembler.completedIngots)
	ingotAssembler.mu.Unlock()

	if len(ingotsToPublish) == 0 {
		t.Fatal("No ingots were assembled from the ores")
	}

	t.Logf("Found %d assembled ingot(s), publishing to NATS...", len(ingotsToPublish))

	// Publish batch to NATS
	if err := mintClient.PublishBatch(ingotsToPublish); err != nil {
		t.Fatalf("failed to publish batch: %v", err)
	}

	// 10. Wait for NATS message
	select {
	case envelope := <-publishedBatches:
		t.Log("Received batch envelope from NATS")

		// Validate batch envelope
		if envelope.BatchID == "" {
			t.Error("batch_id is empty")
		}
		if envelope.Timestamp.IsZero() {
			t.Error("timestamp is zero")
		}
		if envelope.Count != len(ingotsToPublish) {
			t.Errorf("expected count %d, got %d", len(ingotsToPublish), envelope.Count)
		}
		if len(envelope.Ingots) != len(ingotsToPublish) {
			t.Errorf("expected %d ingots in envelope, got %d", len(ingotsToPublish), len(envelope.Ingots))
		}

		// Validate first ingot
		if len(envelope.Ingots) > 0 {
			ingot := envelope.Ingots[0]

			if ingot.JouleTorqTotal != 3600 {
				t.Errorf("expected ingot JouleTorqTotal 3600, got %d", ingot.JouleTorqTotal)
			}
			if ingot.IngotID == "" {
				t.Error("ingot ID is empty")
			}
			if len(ingot.ContractIDs) == 0 {
				t.Error("ingot has no contract IDs")
			} else if ingot.ContractIDs[0] != "integration-test-contract" {
				t.Errorf("expected contract ID 'integration-test-contract', got '%s'", ingot.ContractIDs[0])
			}
			if ingot.RoboStakeTotal == 0 {
				t.Error("robo stake total is zero")
			}
			if ingot.PricePerRT == 0 {
				t.Error("price per RT is zero")
			}

			t.Logf("Ingot validated: joules=%d, contracts=%d, robo_stake=%.5f, price=%.2f, id=%s",
				ingot.JouleTorqTotal,
				len(ingot.ContractIDs),
				ingot.RoboStakeTotal,
				ingot.PricePerRT,
				ingot.IngotID,
			)
		}

		t.Log("✅ Integration test PASSED: Full pipeline working correctly")

	case <-time.After(5 * time.Second):
		t.Fatal("timeout waiting for NATS batch publication")
	}
}

// TestRefineryIntegration_MultipleIngots tests assembling multiple ingots
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
		JouleQueueSize:     1000,
		RoboQueueSize:      1000,
	}

	// Initialize components
	queueManager := NewQueueManager(ctx, cfg.JouleQueueSize, cfg.RoboQueueSize)
	ingotAssembler := NewIngotAssembler(ctx, queueManager)
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

	publishedBatches := make(chan *BatchEnvelope, 10)
	sub, err := nc.Subscribe("mint.ingots", func(msg *nats.Msg) {
		var envelope BatchEnvelope
		if err := json.Unmarshal(msg.Data, &envelope); err != nil {
			t.Errorf("failed to unmarshal batch envelope: %v", err)
			return
		}
		publishedBatches <- &envelope
	})
	if err != nil {
		t.Fatalf("failed to subscribe to NATS: %v", err)
	}
	defer sub.Unsubscribe()

	// Send enough ores for 2 complete ingots (7200 joules total)
	// Plus a partial ingot (1800 joules)
	totalJoules := 9000.0
	joulePerOre := 900.0
	oresNeeded := int(totalJoules / joulePerOre) // 10 ores

	for i := 0; i < oresNeeded; i++ {
		jouleItem := &models.JouleQueueItem{
			Amount:     joulePerOre,
			ContractID: fmt.Sprintf("contract-%d", i%3), // Mix of 3 contracts
			Hash:       fmt.Sprintf("hash-%d", i),
		}
		roboItem := &models.RoboQueueItem{
			Amount:     0.00416,
			Price:      14.42,
			ContractID: jouleItem.ContractID,
		}

		if err := queueManager.AddJoule(jouleItem); err != nil {
			t.Fatalf("failed to add joule %d: %v", i, err)
		}
		if err := queueManager.AddRobo(roboItem); err != nil {
			t.Fatalf("failed to add robo %d: %v", i, err)
		}
	}

	// Wait for ingots to be assembled
	time.Sleep(3 * time.Second)

	// Check completed ingots
	ingotAssembler.mu.Lock()
	completedCount := len(ingotAssembler.completedIngots)
	carriedOverJoules := ingotAssembler.accumulatedJoules
	ingotsToPublish := make([]*models.TokenTorqIngot, len(ingotAssembler.completedIngots))
	copy(ingotsToPublish, ingotAssembler.completedIngots)
	ingotAssembler.mu.Unlock()

	// Should have 2 complete ingots (3600 * 2 = 7200)
	// and 1800 joules carried over
	if completedCount != 2 {
		t.Errorf("expected 2 completed ingots, got %d", completedCount)
	}
	if carriedOverJoules != 1800.0 {
		t.Errorf("expected 1800 joules carried over, got %.2f", carriedOverJoules)
	}

	t.Logf("Assembled %d ingots with %.2f joules carried over", completedCount, carriedOverJoules)

	// Publish batch
	if len(ingotsToPublish) > 0 {
		if err := mintClient.PublishBatch(ingotsToPublish); err != nil {
			t.Fatalf("failed to publish batch: %v", err)
		}

		// Wait for NATS message
		select {
		case envelope := <-publishedBatches:
			if envelope.Count != 2 {
				t.Errorf("expected batch count 2, got %d", envelope.Count)
			}
			t.Logf("✅ Published batch with %d ingots", envelope.Count)

		case <-time.After(5 * time.Second):
			t.Fatal("timeout waiting for NATS batch publication")
		}
	}
}

// TestRefineryIntegration_HTTPValidation tests error handling in ore reception
func TestRefineryIntegration_HTTPValidation(t *testing.T) {
	ctx := context.Background()
	queueManager := NewQueueManager(ctx, 10, 10)
	oreReceiver := NewOreReceiver(queueManager)

	handler := http.HandlerFunc(oreReceiver.HTTPHandler)
	server := httptest.NewServer(handler)
	defer server.Close()

	tests := []struct {
		name           string
		ore            *models.JouleTorqOre
		expectedStatus int
	}{
		{
			name: "valid ore",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-001",
				ContractID:      "contract-001",
				TokensGenerated: 60,
				Joules:          1250,
				MilestoneIndex:  0,
				Timestamp:       uint64(time.Now().Unix()),
				RoboStakeAmount: 0.00416,
			},
			expectedStatus: http.StatusOK,
		},
		{
			name: "missing digger ID",
			ore: &models.JouleTorqOre{
				DiggerID:        "", // Invalid
				ContractID:      "contract-001",
				TokensGenerated: 60,
				Joules:          1250,
				MilestoneIndex:  0,
				Timestamp:       uint64(time.Now().Unix()),
				RoboStakeAmount: 0.00416,
			},
			expectedStatus: http.StatusBadRequest,
		},
		{
			name: "zero joules",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-001",
				ContractID:      "contract-001",
				TokensGenerated: 60,
				Joules:          0, // Invalid
				MilestoneIndex:  0,
				Timestamp:       uint64(time.Now().Unix()),
				RoboStakeAmount: 0.00416,
			},
			expectedStatus: http.StatusBadRequest,
		},
		{
			name: "zero timestamp",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-001",
				ContractID:      "contract-001",
				TokensGenerated: 60,
				Joules:          1250,
				MilestoneIndex:  0,
				Timestamp:       0, // Invalid
				RoboStakeAmount: 0.00416,
			},
			expectedStatus: http.StatusBadRequest,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			oreJSON, _ := json.Marshal(tt.ore)
			resp, err := http.Post(server.URL, "application/json", bytes.NewBuffer(oreJSON))
			if err != nil {
				t.Fatalf("failed to POST ore: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != tt.expectedStatus {
				body, _ := io.ReadAll(resp.Body)
				t.Errorf("expected status %d, got %d - %s", tt.expectedStatus, resp.StatusCode, string(body))
			}
		})
	}
}

// TestRefineryIntegration_QueueBackpressure tests queue full handling
func TestRefineryIntegration_QueueBackpressure(t *testing.T) {
	ctx := context.Background()
	// Create small queues (capacity 2)
	queueManager := NewQueueManager(ctx, 2, 2)
	oreReceiver := NewOreReceiver(queueManager)

	handler := http.HandlerFunc(oreReceiver.HTTPHandler)
	server := httptest.NewServer(handler)
	defer server.Close()

	// Send 3 ores (should fill queue and reject 3rd)
	for i := 0; i < 3; i++ {
		ore := &models.JouleTorqOre{
			DiggerID:        fmt.Sprintf("digger-%d", i),
			ContractID:      "contract-001",
			TokensGenerated: 60,
			Joules:          1250,
			MilestoneIndex:  uint32(i),
			Timestamp:       uint64(time.Now().Unix()),
			RoboStakeAmount: 0.00416,
		}

		oreJSON, _ := json.Marshal(ore)
		resp, err := http.Post(server.URL, "application/json", bytes.NewBuffer(oreJSON))
		if err != nil {
			t.Fatalf("failed to POST ore %d: %v", i, err)
		}

		body, _ := io.ReadAll(resp.Body)
		resp.Body.Close()

		if i < 2 {
			// First 2 should succeed
			if resp.StatusCode != http.StatusOK {
				t.Errorf("ore %d: expected status 200, got %d - %s", i, resp.StatusCode, string(body))
			}
		} else {
			// 3rd should be rejected with 429 (queue full)
			if resp.StatusCode != http.StatusTooManyRequests {
				t.Errorf("ore %d: expected status 429 (queue full), got %d - %s", i, resp.StatusCode, string(body))
			}
		}
	}
}

// BatchEnvelope matches the structure in mint_client.go
type BatchEnvelope struct {
	BatchID   string                   `json:"batch_id"`
	Timestamp time.Time                `json:"timestamp"`
	Count     int                      `json:"count"`
	Ingots    []*models.TokenTorqIngot `json:"ingots"`
}
