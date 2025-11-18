// models_test.go - Tests for data models
package distodam

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestValidateMintEvent_Valid(t *testing.T) {
	event := &MintEvent{
		BatchHash: "abc123",
		IngotStakes: []IngotStake{
			{
				IngotID:        "ingot-001",
				RoboStakeTotal: 0.05,
				ContractIDs:    []string{"contract-001"},
			},
		},
		IngotsProcessed: 1000,
		BatchID:         "batch-001",
		Timestamp:       time.Now(),
	}

	err := ValidateMintEvent(event)
	assert.NoError(t, err)
}

func TestValidateMintEvent_Nil(t *testing.T) {
	err := ValidateMintEvent(nil)
	assert.ErrorIs(t, err, ErrNilEvent)
}

func TestValidateMintEvent_MissingBatchID(t *testing.T) {
	event := &MintEvent{
		BatchHash: "abc123",
		IngotStakes: []IngotStake{
			{IngotID: "ingot-001", RoboStakeTotal: 0.05},
		},
		// BatchID missing
	}

	err := ValidateMintEvent(event)
	assert.ErrorIs(t, err, ErrMissingBatchID)
}

func TestValidateMintEvent_NoIngotStakes(t *testing.T) {
	event := &MintEvent{
		BatchHash:   "abc123",
		BatchID:     "batch-001",
		IngotStakes: []IngotStake{}, // Empty
	}

	err := ValidateMintEvent(event)
	assert.ErrorIs(t, err, ErrNoIngotStakes)
}

func TestValidateContract_Valid(t *testing.T) {
	contract := &Contract{
		ID:         "contract-001",
		TrustID:    "trust-001",
		Builder:    "builder-001",
		RoboStake:  1.5,
		Status:     "approved",
		ApprovedAt: time.Now(),
		ApprovedBy: "bidnet-001",
	}

	err := ValidateContract(contract)
	assert.NoError(t, err)
}

func TestValidateContract_Nil(t *testing.T) {
	err := ValidateContract(nil)
	assert.ErrorIs(t, err, ErrNilContract)
}

func TestValidateContract_MissingID(t *testing.T) {
	contract := &Contract{
		// ID missing
		RoboStake: 1.5,
		Status:    "approved",
	}

	err := ValidateContract(contract)
	assert.ErrorIs(t, err, ErrMissingContractID)
}

func TestValidateContract_InvalidStatus(t *testing.T) {
	contract := &Contract{
		ID:        "contract-001",
		RoboStake: 1.5,
		Status:    "pending", // Not "approved"
	}

	err := ValidateContract(contract)
	assert.ErrorIs(t, err, ErrInvalidStatus)
}

func TestValidateContract_ZeroRoboStake(t *testing.T) {
	contract := &Contract{
		ID:        "contract-001",
		RoboStake: 0, // Invalid
		Status:    "approved",
	}

	err := ValidateContract(contract)
	assert.ErrorIs(t, err, ErrInvalidRoboStake)
}

func TestValidateContract_NegativeRoboStake(t *testing.T) {
	contract := &Contract{
		ID:        "contract-001",
		RoboStake: -1.5, // Invalid
		Status:    "approved",
	}

	err := ValidateContract(contract)
	assert.ErrorIs(t, err, ErrInvalidRoboStake)
}

func TestIngotStake_Serialization(t *testing.T) {
	stake := IngotStake{
		IngotID:        "ingot-abc-123",
		RoboStakeTotal: 0.05432,
		ContractIDs:    []string{"contract-001", "contract-002"},
	}

	assert.Equal(t, "ingot-abc-123", stake.IngotID)
	assert.Equal(t, 0.05432, stake.RoboStakeTotal)
	assert.Len(t, stake.ContractIDs, 2)
	assert.Contains(t, stake.ContractIDs, "contract-001")
	assert.Contains(t, stake.ContractIDs, "contract-002")
}

func TestContract_FundingFields(t *testing.T) {
	contract := &Contract{
		ID:        "contract-001",
		RoboStake: 1.5,
		Status:    "approved",
	}

	// Initially unfunded
	assert.Empty(t, contract.FundedBy)
	assert.Empty(t, contract.LoanID)
	assert.Empty(t, contract.VaultSource)

	// Simulate funding
	contract.FundedAt = time.Now()
	contract.FundedBy = "distodam-001"
	contract.VaultSource = "stake_vault"
	contract.Status = "funded"

	assert.NotEmpty(t, contract.FundedAt)
	assert.Equal(t, "distodam-001", contract.FundedBy)
	assert.Equal(t, "stake_vault", contract.VaultSource)
	assert.Equal(t, "funded", contract.Status)
}

func TestContract_FundingWithLoan(t *testing.T) {
	contract := &Contract{
		ID:        "contract-002",
		RoboStake: 2.0,
		Status:    "approved",
	}

	// Simulate funding via loan
	contract.FundedAt = time.Now()
	contract.FundedBy = "distodam-002"
	contract.VaultSource = "disto_vault_loan"
	contract.LoanID = "loan-abc-123"
	contract.Status = "funded"

	assert.Equal(t, "disto_vault_loan", contract.VaultSource)
	assert.Equal(t, "loan-abc-123", contract.LoanID)
	assert.NotEmpty(t, contract.FundedAt)
}

func TestValidationError_ErrorMessage(t *testing.T) {
	err := &ValidationError{
		Field:  "test_field",
		Reason: "test reason",
	}

	expected := "validation error: test_field - test reason"
	assert.Equal(t, expected, err.Error())
}
