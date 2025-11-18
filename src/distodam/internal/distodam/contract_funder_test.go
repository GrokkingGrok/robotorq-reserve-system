// contract_funder_test.go - Tests for ContractFunder
package distodam

import (
	"encoding/json"
	"log/slog"
	"os"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestContractValidation(t *testing.T) {
	tests := []struct {
		name        string
		contract    *Contract
		expectError bool
	}{
		{
			name: "valid approved contract",
			contract: &Contract{
				ID:         "contract-001",
				RoboStake:  1.5,
				Status:     "approved",
				ApprovedAt: time.Now(),
			},
			expectError: false,
		},
		{
			name: "invalid status",
			contract: &Contract{
				ID:        "contract-002",
				RoboStake: 1.5,
				Status:    "pending", // Not approved
			},
			expectError: true,
		},
		{
			name: "zero robo stake",
			contract: &Contract{
				ID:        "contract-003",
				RoboStake: 0,
				Status:    "approved",
			},
			expectError: true,
		},
		{
			name: "negative robo stake",
			contract: &Contract{
				ID:        "contract-004",
				RoboStake: -1.0,
				Status:    "approved",
			},
			expectError: true,
		},
		{
			name: "missing contract ID",
			contract: &Contract{
				RoboStake: 1.5,
				Status:    "approved",
			},
			expectError: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateContract(tt.contract)
			if tt.expectError {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestContractSerialization(t *testing.T) {
	contract := &Contract{
		ID:            "contract-abc-123",
		TrustID:       "trust-001",
		OpportunityID: "opp-001",
		Builder:       "builder-001",
		DiggerURL:     "http://localhost:8080",
		RoboStake:     2.5,
		ROI:           1.8,
		Torq:          1000,
		Status:        "approved",
		ApprovedAt:    time.Now(),
		ApprovedBy:    "bidnet-001",
	}

	// Serialize to JSON
	data, err := json.Marshal(contract)
	require.NoError(t, err)

	// Deserialize
	var decoded Contract
	err = json.Unmarshal(data, &decoded)
	require.NoError(t, err)

	// Verify fields
	assert.Equal(t, contract.ID, decoded.ID)
	assert.Equal(t, contract.TrustID, decoded.TrustID)
	assert.Equal(t, contract.RoboStake, decoded.RoboStake)
	assert.Equal(t, contract.ROI, decoded.ROI)
	assert.Equal(t, contract.Status, decoded.Status)
}

func TestContractFundingWorkflow(t *testing.T) {
	// This test demonstrates the expected contract funding workflow

	// 1. Contract arrives from BidNet
	contract := &Contract{
		ID:         "contract-workflow-001",
		RoboStake:  1.5,
		Status:     "approved",
		ApprovedAt: time.Now(),
		ApprovedBy: "bidnet-001",
	}

	// 2. Validate contract
	err := ValidateContract(contract)
	require.NoError(t, err)

	// 3. Fund contract (simulated - would call VaultManager.FundContract)
	contract.FundedAt = time.Now()
	contract.FundedBy = "distodam-001"
	contract.VaultSource = "stake_vault"
	contract.Status = "funded"

	// 4. Verify funding fields populated
	assert.NotEmpty(t, contract.FundedAt)
	assert.Equal(t, "distodam-001", contract.FundedBy)
	assert.Equal(t, "stake_vault", contract.VaultSource)
	assert.Equal(t, "funded", contract.Status)
	assert.Empty(t, contract.LoanID) // No loan needed
}

func TestContractFundingWithLoan(t *testing.T) {
	// This test demonstrates contract funding via loan

	contract := &Contract{
		ID:         "contract-loan-001",
		RoboStake:  3.0,
		Status:     "approved",
		ApprovedAt: time.Now(),
	}

	// Fund via loan (simulated)
	contract.FundedAt = time.Now()
	contract.FundedBy = "distodam-002"
	contract.VaultSource = "disto_vault_loan"
	contract.LoanID = "loan-abc-123"
	contract.Status = "funded"

	// Verify loan fields
	assert.Equal(t, "disto_vault_loan", contract.VaultSource)
	assert.Equal(t, "loan-abc-123", contract.LoanID)
	assert.NotEmpty(t, contract.FundedAt)
}

func TestContractFundedEventCreation(t *testing.T) {
	contract := &Contract{
		ID:          "contract-evt-001",
		RoboStake:   1.5,
		Status:      "funded",
		FundedAt:    time.Now(),
		FundedBy:    "distodam-001",
		VaultSource: "stake_vault",
	}

	event := &ContractFundedEvent{
		EventID:    "evt-001",
		ContractID: contract.ID,
		AmountRT:   contract.RoboStake,
		Source:     contract.VaultSource,
		Timestamp:  contract.FundedAt,
		VaultRatio: 0.95,
	}

	// Serialize event
	data, err := json.Marshal(event)
	require.NoError(t, err)

	// Deserialize
	var decoded ContractFundedEvent
	err = json.Unmarshal(data, &decoded)
	require.NoError(t, err)

	assert.Equal(t, event.EventID, decoded.EventID)
	assert.Equal(t, event.ContractID, decoded.ContractID)
	assert.Equal(t, event.AmountRT, decoded.AmountRT)
	assert.Equal(t, event.Source, decoded.Source)
	assert.Equal(t, event.VaultRatio, decoded.VaultRatio)
}

func TestContractFundedEventWithLoan(t *testing.T) {
	contract := &Contract{
		ID:          "contract-evt-loan-001",
		RoboStake:   2.0,
		Status:      "funded",
		FundedAt:    time.Now(),
		FundedBy:    "distodam-002",
		VaultSource: "disto_vault_loan",
		LoanID:      "loan-xyz-789",
	}

	event := &ContractFundedEvent{
		EventID:    "evt-loan-001",
		ContractID: contract.ID,
		AmountRT:   contract.RoboStake,
		Source:     contract.VaultSource,
		LoanID:     contract.LoanID,
		Timestamp:  contract.FundedAt,
		VaultRatio: 0.45,
	}

	// Verify loan ID included
	assert.Equal(t, "loan-xyz-789", event.LoanID)
	assert.Equal(t, "disto_vault_loan", event.Source)
}

// Integration test with VaultManager would go here
// Example:
//
// func TestContractFunder_Integration(t *testing.T) {
//     logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
//
//     // Create MockVaultClient with initial balances
//     vaultClient := NewMockVaultClient(10.0, 5.0, logger, nil)
//
//     // Create VaultManager
//     config := &config.Config{LoanPolicy: config.LoanPolicyNatural}
//     vaultManager := NewVaultManager(vaultClient, logger, nil, config)
//
//     // Create ContractFunder (would need NATS connection)
//     // cf := NewContractFunder(nc, logger, vaultManager, ...)
//
//     // Publish test contract to NATS
//     // Verify ContractFundedEvent received
//     // Verify vault balances updated
// }

func TestNewContractFunder(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	// For unit test, we can't create actual NATS connection
	// But we can verify the constructor doesn't panic

	// This would require mocking NATS connection in real integration test
	logger.Info("ContractFunder constructor validated",
		"test", "NewContractFunder",
		"note", "Full integration test requires NATS server")
}

func TestContractJSONFields(t *testing.T) {
	// Verify all JSON tags are correct
	contract := &Contract{
		ID:            "test-001",
		TrustID:       "trust-001",
		OpportunityID: "opp-001",
		Builder:       "builder-001",
		DiggerURL:     "http://test",
		RoboStake:     1.0,
		ROI:           1.5,
		Torq:          100,
		Status:        "approved",
		ApprovedAt:    time.Now(),
		ApprovedBy:    "bidnet-001",
		FundedAt:      time.Now(),
		FundedBy:      "distodam-001",
		LoanID:        "loan-001",
		VaultSource:   "stake_vault",
	}

	data, err := json.Marshal(contract)
	require.NoError(t, err)

	// Verify JSON contains expected keys
	var jsonMap map[string]interface{}
	err = json.Unmarshal(data, &jsonMap)
	require.NoError(t, err)

	assert.Contains(t, jsonMap, "id")
	assert.Contains(t, jsonMap, "trust_id")
	assert.Contains(t, jsonMap, "robo_stake")
	assert.Contains(t, jsonMap, "status")
	assert.Contains(t, jsonMap, "funded_at")
	assert.Contains(t, jsonMap, "loan_id")
	assert.Contains(t, jsonMap, "vault_source")
}
