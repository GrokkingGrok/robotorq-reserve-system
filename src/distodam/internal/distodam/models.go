// models.go - Data structures for DistoDam event processing
package distodam

import "time"

// Phase3RoboTorqUnit represents a 1 RT unit from Mint (Phase 3)
// Published to: distodam.units
type Phase3RoboTorqUnit struct {
	UnitID         string    `json:"unit_id"`
	MerkleRoot     string    `json:"merkle_root"`
	TreeHeight     int       `json:"tree_height"`
	RoboStakeTotal float64   `json:"robo_stake_total"`
	ContractIDs    []string  `json:"contract_ids"`
	DiggerIDs      []string  `json:"digger_ids"`
	MerkleProofAPI string    `json:"merkle_proof_api"`
	MintedAt       time.Time `json:"minted_at"`
	Signature      string    `json:"signature,omitempty"`
	PublicKey      string    `json:"public_key,omitempty"`
}

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

// UBDWalletRegistration represents a wallet registered for UBD distributions
// Published to: ubd.wallet.registered (from Wallet service - future)
type UBDWalletRegistration struct {
	WalletID     string    `json:"wallet_id"`     // Wallet address
	MemberID     string    `json:"member_id"`     // Member identifier
	RegisteredAt time.Time `json:"registered_at"` // Registration timestamp
	Active       bool      `json:"active"`        // Whether wallet is active for distributions
}

// UBDDistributionEvent represents a UBD payment event
// Published to: ubd.distributed (from DistoDam - future)
// This is what DistoDam publishes when it distributes from DistoVault
type UBDDistributionEvent struct {
	EventID      string    `json:"event_id"`      // Unique event identifier
	WalletID     string    `json:"wallet_id"`     // Recipient wallet
	AmountRT     float64   `json:"amount_rt"`     // Amount distributed
	Timestamp    time.Time `json:"timestamp"`     // Distribution time
	DistoBalance float64   `json:"disto_balance"` // DistoVault balance after distribution
	Reason       string    `json:"reason"`        // "periodic_distribution", "manual", etc.
}

// WalletActivationMessage represents a wallet requesting distribution activation
// Published to: wallet.activate (from Wallet service)
type WalletActivationMessage struct {
	WalletID    string    `json:"wallet_id"`    // Unique wallet identifier
	Activate    bool      `json:"activate"`     // true = activate, false = deactivate
	RequestedAt time.Time `json:"requested_at"` // When activation was requested
}

// WalletDistributionMessage represents RT distribution to a wallet
// Published to: wallet.distribution (from DistoDam to Wallet)
type WalletDistributionMessage struct {
	WalletID     string             `json:"wallet_id"`     // Recipient wallet
	RTUnit       Phase3RoboTorqUnit `json:"rt_unit"`       // The actual RT certificate!
	Timestamp    time.Time          `json:"timestamp"`     // When distributed
	DistoBalance float64            `json:"disto_balance"` // DistoVault balance after withdrawal
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
