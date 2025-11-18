// models.go - Data structures for DistoDam event processing
package distodam

import "time"

// MintEvent represents a batch of processed ingots from the Mint service
// Published to: mint.batches
type MintEvent struct {
	// Cryptographic proof
	BatchHash string `json:"batch_hash"` // SHA256 of batch data (future: Merkle root)

	// Individual ingot stakes (each ingot's RoboStake returns separately)
	IngotStakes []IngotStake `json:"ingot_stakes"` // Array of stakes, preserves granularity

	// Metadata
	IngotsProcessed int       `json:"ingots_count"` // Batch size (usually 1000)
	BatchID         string    `json:"batch_id"`     // Unique identifier
	Timestamp       time.Time `json:"timestamp"`    // UTC timestamp
}

// IngotStake represents the RoboStake from a single ingot
// CRITICAL: Each ingot stake is processed separately (not summed)
type IngotStake struct {
	IngotID        string   `json:"ingot_id"`         // Unique ingot identifier
	RoboStakeTotal float64  `json:"robo_stake_total"` // RoboStake for THIS ingot
	ContractIDs    []string `json:"contract_ids"`     // Contracts that contributed to this ingot
}

// Contract represents an approved contract awaiting funding
// Published to: contracts.approved (from BidNet)
type Contract struct {
	// Core fields (from Trust service)
	ID            string  `json:"id"`
	TrustID       string  `json:"trust_id"`
	OpportunityID string  `json:"opportunity_id"`
	Builder       string  `json:"builder"`
	DiggerURL     string  `json:"digger_url"`
	RoboStake     float64 `json:"robo_stake"` // Amount to fund (critical field)
	ROI           float64 `json:"roi"`        // Expected return on investment
	Torq          int     `json:"torq"`       // Computational complexity

	// BidNet evaluation fields
	Status     string    `json:"status"`      // "approved", "funded", "rejected"
	ApprovedAt time.Time `json:"approved_at"` // When BidNet approved
	ApprovedBy string    `json:"approved_by"` // BidNet instance ID

	// DistoDam funding fields (added by DistoDam)
	FundedAt    time.Time `json:"funded_at,omitempty"`    // When funding occurred
	FundedBy    string    `json:"funded_by,omitempty"`    // DistoDam instance ID
	LoanID      string    `json:"loan_id,omitempty"`      // If funded via loan
	VaultSource string    `json:"vault_source,omitempty"` // "stake_vault" or "disto_vault_loan"
}

// UBDRequest represents a UBD distribution request
// Published to: ubd.requests (from Wallet service - future)
type UBDRequest struct {
	RequestID   string    `json:"request_id"`   // Unique request identifier
	RecipientID string    `json:"recipient_id"` // Member wallet ID
	AmountRT    float64   `json:"amount_rt"`    // Requested RT amount
	Timestamp   time.Time `json:"timestamp"`    // Request time
	Reason      string    `json:"reason"`       // "monthly_ubi", "bonus", "adjustment"
}

// ValidateMintEvent checks if a MintEvent is valid for processing
func ValidateMintEvent(event *MintEvent) error {
	if event == nil {
		return ErrNilEvent
	}
	if event.BatchID == "" {
		return ErrMissingBatchID
	}
	if len(event.IngotStakes) == 0 {
		return ErrNoIngotStakes
	}
	return nil
}

// ValidateContract checks if a Contract is valid for funding
func ValidateContract(contract *Contract) error {
	if contract == nil {
		return ErrNilContract
	}
	if contract.ID == "" {
		return ErrMissingContractID
	}
	if contract.Status != "approved" {
		return ErrInvalidStatus
	}
	if contract.RoboStake <= 0 {
		return ErrInvalidRoboStake
	}
	return nil
}

// Validation errors
var (
	ErrNilEvent          = &ValidationError{Field: "event", Reason: "event is nil"}
	ErrMissingBatchID    = &ValidationError{Field: "batch_id", Reason: "batch_id is required"}
	ErrNoIngotStakes     = &ValidationError{Field: "ingot_stakes", Reason: "at least one ingot stake required"}
	ErrNilContract       = &ValidationError{Field: "contract", Reason: "contract is nil"}
	ErrMissingContractID = &ValidationError{Field: "id", Reason: "contract id is required"}
	ErrInvalidStatus     = &ValidationError{Field: "status", Reason: "status must be 'approved'"}
	ErrInvalidRoboStake  = &ValidationError{Field: "robo_stake", Reason: "robo_stake must be positive"}
)

// ValidationError represents a validation failure
type ValidationError struct {
	Field  string
	Reason string
}

func (e *ValidationError) Error() string {
	return "validation error: " + e.Field + " - " + e.Reason
}
