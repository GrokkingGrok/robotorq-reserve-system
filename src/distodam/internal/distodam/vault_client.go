// Package distodam provides core DistoDam vault coordination logic.
package distodam

// VaultClient defines the interface for vault operations.
// This abstraction allows swapping between MockVaultClient (Phase 6)
// and NATSVaultClient (Phase 7+) with minimal code changes.
//
// Phase 6: Internal atomic.Int64 state
// Phase 7+: NATS commands to distributed shadow vault network
type VaultClient interface {
	// Deposit operations

	// DepositToStakeVault deposits RoboTorq to the StakeVault.
	// Used when circulating RoboStake returns from Mint (proof chain completion).
	DepositToStakeVault(amountRT float64) error

	// DepositToDistoVault deposits newly minted RoboTorq to the DistoVault.
	// Used for Torq markup (value-add from production).
	DepositToDistoVault(amountRT float64) error

	// Withdrawal operations

	// WithdrawFromStakeVault attempts to withdraw RoboTorq from StakeVault for contract funding.
	// Returns (success, error):
	//   - success=true: Withdrawal successful, contract funded
	//   - success=false, error=nil: Insufficient balance (not an error, triggers loan logic)
	//   - success=false, error!=nil: Actual error (vault unavailable, etc.)
	WithdrawFromStakeVault(contractID string, amountRT float64) (success bool, err error)

	// WithdrawFromDistoVault attempts to withdraw RoboTorq from DistoVault for loan to StakeVault.
	// Returns (success, error) with same semantics as WithdrawFromStakeVault.
	WithdrawFromDistoVault(loanID string, amountRT float64) (success bool, err error)

	// Query operations

	// GetStakeVaultBalance returns the current StakeVault balance in RT.
	GetStakeVaultBalance() (float64, error)

	// GetDistoVaultBalance returns the current DistoVault balance in RT.
	GetDistoVaultBalance() (float64, error)

	// Loan repayment

	// RepayLoan repays a loan from StakeVault back to DistoVault.
	// Called when RoboStake returns from Mint and loans are outstanding.
	RepayLoan(loanID string, amountRT float64) error
}
