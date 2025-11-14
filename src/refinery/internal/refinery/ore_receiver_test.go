package refinery

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"b2b/refinery/internal/models"
)

// mockQueueManager implements QueueAdder for testing
type mockQueueManager struct {
	jouleItems []*models.JouleQueueItem
	roboItems  []*models.RoboQueueItem
	shouldFail bool
}

func (m *mockQueueManager) AddJoule(item *models.JouleQueueItem) error {
	if m.shouldFail {
		return models.ErrQueueFull
	}
	m.jouleItems = append(m.jouleItems, item)
	return nil
}

func (m *mockQueueManager) AddRobo(item *models.RoboQueueItem) error {
	if m.shouldFail {
		return models.ErrQueueFull
	}
	m.roboItems = append(m.roboItems, item)
	return nil
}

func TestOreReceiver_ReceiveOre(t *testing.T) {
	tests := []struct {
		name          string
		ore           *models.JouleTorqOre
		queueFull     bool
		wantErr       bool
		expectedJoule float64
		expectedRobo  float64
		expectedPrice float64
	}{
		{
			name: "valid ore with all fields",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull:     false,
			wantErr:       false,
			expectedJoule: 750.0,
			expectedRobo:  100.0,
			expectedPrice: 10.0, // 1000 tokens / 100 robo = 10
		},
		{
			name: "valid ore with minimal robo stake",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-789",
				ContractID:      "contract-101",
				TokensGenerated: 500,
				Joules:          1000,
				MilestoneIndex:  1,
				Timestamp:       1699999999,
				RoboStakeAmount: 50.0,
			},
			queueFull:     false,
			wantErr:       false,
			expectedJoule: 1000.0,
			expectedRobo:  50.0,
			expectedPrice: 10.0, // 500 / 50 = 10
		},
		{
			name: "invalid ore - missing digger_id",
			ore: &models.JouleTorqOre{
				DiggerID:        "", // Missing
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull: false,
			wantErr:   true,
		},
		{
			name: "invalid ore - missing contract_id",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "", // Missing
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull: false,
			wantErr:   true,
		},
		{
			name: "invalid ore - zero joules",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          0, // Invalid
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull: false,
			wantErr:   true,
		},
		{
			name: "invalid ore - negative robo stake",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: -10.0, // Invalid
			},
			queueFull: false,
			wantErr:   true,
		},
		{
			name: "queue full - should return error",
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull: true,
			wantErr:   true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Create mock queue manager
			mockQueue := &mockQueueManager{
				shouldFail: tt.queueFull,
			}

			// Create ore receiver
			receiver := NewOreReceiver(mockQueue)

			// Call ReceiveOre
			err := receiver.ReceiveOre(tt.ore)

			// Check error expectation
			if (err != nil) != tt.wantErr {
				t.Errorf("ReceiveOre() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			// If error expected, skip validation checks
			if tt.wantErr {
				return
			}

			// Validate joule item was added correctly
			if len(mockQueue.jouleItems) != 1 {
				t.Errorf("Expected 1 joule item, got %d", len(mockQueue.jouleItems))
				return
			}

			jouleItem := mockQueue.jouleItems[0]
			if jouleItem.Amount != tt.expectedJoule {
				t.Errorf("Joule amount = %v, want %v", jouleItem.Amount, tt.expectedJoule)
			}
			if jouleItem.ContractID != tt.ore.ContractID {
				t.Errorf("Joule ContractID = %v, want %v", jouleItem.ContractID, tt.ore.ContractID)
			}
			if jouleItem.Hash == "" {
				t.Error("Joule Hash should not be empty")
			}

			// Validate robo item was added correctly
			if len(mockQueue.roboItems) != 1 {
				t.Errorf("Expected 1 robo item, got %d", len(mockQueue.roboItems))
				return
			}

			roboItem := mockQueue.roboItems[0]
			if roboItem.Amount != tt.expectedRobo {
				t.Errorf("Robo amount = %v, want %v", roboItem.Amount, tt.expectedRobo)
			}
			if roboItem.Price != tt.expectedPrice {
				t.Errorf("Robo price = %v, want %v", roboItem.Price, tt.expectedPrice)
			}
			if roboItem.ContractID != tt.ore.ContractID {
				t.Errorf("Robo ContractID = %v, want %v", roboItem.ContractID, tt.ore.ContractID)
			}
		})
	}
}

func TestOreReceiver_HTTPHandler(t *testing.T) {
	tests := []struct {
		name           string
		method         string
		ore            *models.JouleTorqOre
		queueFull      bool
		invalidJSON    bool
		expectedStatus int
		checkResponse  bool
	}{
		{
			name:   "valid POST request",
			method: http.MethodPost,
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull:      false,
			invalidJSON:    false,
			expectedStatus: http.StatusOK,
			checkResponse:  true,
		},
		{
			name:           "invalid method - GET",
			method:         http.MethodGet,
			ore:            nil,
			queueFull:      false,
			invalidJSON:    false,
			expectedStatus: http.StatusMethodNotAllowed,
			checkResponse:  false,
		},
		{
			name:           "invalid method - PUT",
			method:         http.MethodPut,
			ore:            nil,
			queueFull:      false,
			invalidJSON:    false,
			expectedStatus: http.StatusMethodNotAllowed,
			checkResponse:  false,
		},
		{
			name:           "invalid JSON",
			method:         http.MethodPost,
			ore:            nil,
			queueFull:      false,
			invalidJSON:    true,
			expectedStatus: http.StatusBadRequest,
			checkResponse:  false,
		},
		{
			name:   "queue full - should return 429",
			method: http.MethodPost,
			ore: &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull:      true,
			invalidJSON:    false,
			expectedStatus: http.StatusTooManyRequests,
			checkResponse:  false,
		},
		{
			name:   "invalid ore - missing digger_id",
			method: http.MethodPost,
			ore: &models.JouleTorqOre{
				DiggerID:        "", // Missing
				ContractID:      "contract-456",
				TokensGenerated: 1000,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: 100.0,
			},
			queueFull:      false,
			invalidJSON:    false,
			expectedStatus: http.StatusBadRequest,
			checkResponse:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Create mock queue manager
			mockQueue := &mockQueueManager{
				shouldFail: tt.queueFull,
			}

			// Create ore receiver
			receiver := NewOreReceiver(mockQueue)

			// Create request
			var req *http.Request
			if tt.invalidJSON {
				// Send invalid JSON
				req = httptest.NewRequest(tt.method, "/receive-ore", bytes.NewBufferString("{invalid json}"))
			} else if tt.ore != nil {
				// Send valid ore JSON
				oreJSON, _ := json.Marshal(tt.ore)
				req = httptest.NewRequest(tt.method, "/receive-ore", bytes.NewBuffer(oreJSON))
			} else {
				// No body
				req = httptest.NewRequest(tt.method, "/receive-ore", nil)
			}

			// Create response recorder
			rr := httptest.NewRecorder()

			// Call handler
			receiver.HTTPHandler(rr, req)

			// Check status code
			if rr.Code != tt.expectedStatus {
				t.Errorf("HTTPHandler() status = %v, want %v", rr.Code, tt.expectedStatus)
			}

			// Check response body for successful requests
			if tt.checkResponse {
				var response map[string]interface{}
				if err := json.NewDecoder(rr.Body).Decode(&response); err != nil {
					t.Errorf("Failed to decode response: %v", err)
					return
				}

				if response["status"] != "accepted" {
					t.Errorf("Response status = %v, want 'accepted'", response["status"])
				}
				if response["contract_id"] != tt.ore.ContractID {
					t.Errorf("Response contract_id = %v, want %v", response["contract_id"], tt.ore.ContractID)
				}
				if response["milestone"] != float64(tt.ore.MilestoneIndex) {
					t.Errorf("Response milestone = %v, want %v", response["milestone"], tt.ore.MilestoneIndex)
				}
			}
		})
	}
}

func TestOreReceiver_PriceCalculation(t *testing.T) {
	tests := []struct {
		name          string
		tokens        uint64
		roboStake     float64
		expectedPrice float64
	}{
		{
			name:          "normal ratio",
			tokens:        1000,
			roboStake:     100.0,
			expectedPrice: 10.0,
		},
		{
			name:          "high token ratio",
			tokens:        5000,
			roboStake:     50.0,
			expectedPrice: 100.0,
		},
		{
			name:          "low token ratio",
			tokens:        100,
			roboStake:     200.0,
			expectedPrice: 0.5,
		},
		{
			name:          "fractional result",
			tokens:        750,
			roboStake:     100.0,
			expectedPrice: 7.5,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Create mock queue manager
			mockQueue := &mockQueueManager{}

			// Create ore receiver
			receiver := NewOreReceiver(mockQueue)

			// Create ore
			ore := &models.JouleTorqOre{
				DiggerID:        "digger-123",
				ContractID:      "contract-456",
				TokensGenerated: tt.tokens,
				Joules:          750,
				MilestoneIndex:  5,
				Timestamp:       1699999999,
				RoboStakeAmount: tt.roboStake,
			}

			// Process ore
			err := receiver.ReceiveOre(ore)
			if err != nil {
				t.Fatalf("Unexpected error: %v", err)
			}

			// Check calculated price
			if len(mockQueue.roboItems) != 1 {
				t.Fatalf("Expected 1 robo item, got %d", len(mockQueue.roboItems))
			}

			actualPrice := mockQueue.roboItems[0].Price
			if actualPrice != tt.expectedPrice {
				t.Errorf("Price = %v, want %v", actualPrice, tt.expectedPrice)
			}
		})
	}
}

func TestOreReceiver_HashGeneration(t *testing.T) {
	// Create mock queue manager
	mockQueue := &mockQueueManager{}

	// Create ore receiver
	receiver := NewOreReceiver(mockQueue)

	// Create two identical ore payloads
	ore1 := &models.JouleTorqOre{
		DiggerID:        "digger-123",
		ContractID:      "contract-456",
		TokensGenerated: 1000,
		Joules:          750,
		MilestoneIndex:  5,
		Timestamp:       1699999999,
		RoboStakeAmount: 100.0,
	}

	ore2 := &models.JouleTorqOre{
		DiggerID:        "digger-123",
		ContractID:      "contract-456",
		TokensGenerated: 1000,
		Joules:          750,
		MilestoneIndex:  5,
		Timestamp:       1699999999,
		RoboStakeAmount: 100.0,
	}

	// Process both ores
	if err := receiver.ReceiveOre(ore1); err != nil {
		t.Fatalf("Failed to process ore1: %v", err)
	}
	if err := receiver.ReceiveOre(ore2); err != nil {
		t.Fatalf("Failed to process ore2: %v", err)
	}

	// Check that hashes are identical for identical data
	if len(mockQueue.jouleItems) != 2 {
		t.Fatalf("Expected 2 joule items, got %d", len(mockQueue.jouleItems))
	}

	hash1 := mockQueue.jouleItems[0].Hash
	hash2 := mockQueue.jouleItems[1].Hash

	if hash1 != hash2 {
		t.Errorf("Identical ore should produce identical hashes: %s != %s", hash1, hash2)
	}

	// Verify hash is hex-encoded SHA256 (64 characters)
	if len(hash1) != 64 {
		t.Errorf("Hash length = %d, want 64 (SHA256 hex)", len(hash1))
	}

	// Create different ore
	ore3 := &models.JouleTorqOre{
		DiggerID:        "digger-789", // Different
		ContractID:      "contract-456",
		TokensGenerated: 1000,
		Joules:          750,
		MilestoneIndex:  5,
		Timestamp:       1699999999,
		RoboStakeAmount: 100.0,
	}

	if err := receiver.ReceiveOre(ore3); err != nil {
		t.Fatalf("Failed to process ore3: %v", err)
	}

	hash3 := mockQueue.jouleItems[2].Hash

	// Different ore should produce different hash
	if hash1 == hash3 {
		t.Error("Different ore should produce different hashes")
	}
}
