package distodam

import (
	"fmt"
	"log/slog"
	"sync"
	"time"

	"b2b/distorouter/internal/config"

	"github.com/google/uuid"
)

// Loan represents a temporary transfer from DistoVault to StakeVault.
// Loans are created when StakeVault has insufficient balance to fund a contract.
// They are repaid FIFO when RoboStake returns from Mint.
type Loan struct {
	LoanID            string
	AmountRT          float64
	BorrowedAt        time.Time
	ContractID        string
	Outstanding       float64 // Remaining balance to be repaid
	Repaid            float64 // Amount repaid so far
	TorqBumpRequested bool    // Policy lever flag
}

// VaultManager orchestrates dual-vault operations and loan management.
// It coordinates StakeVault (circulating RoboStake) and DistoVault (newly minted RT).
//
// Key responsibilities:
//   - Fund contracts (try StakeVault first, borrow from DistoVault if needed)
//   - Track outstanding loans in FIFO order
//   - Repay loans when RoboStake returns from Mint
//   - Apply loan policy lever (natural equilibrium by default)
type VaultManager struct {
	vaultClient VaultClient
	config      *config.Config
	logger      *slog.Logger
	metrics     *VaultMetrics

	// Loan tracking (FIFO)
	loans     []*Loan        // FIFO slice - preserves insertion order
	loanIndex map[string]int // LoanID → index in slice for O(1) lookup
	loansMu   sync.RWMutex
}

// NewVaultManager creates a new VaultManager.
func NewVaultManager(
	vaultClient VaultClient,
	cfg *config.Config,
	logger *slog.Logger,
	metrics *VaultMetrics,
) *VaultManager {
	return &VaultManager{
		vaultClient: vaultClient,
		config:      cfg,
		logger:      logger,
		metrics:     metrics,
		loans:       make([]*Loan, 0),
		loanIndex:   make(map[string]int),
	}
}

// FundContract attempts to fund a contract with the requested amount.
// Algorithm:
//  1. Try to withdraw from StakeVault (normal path)
//  2. If insufficient, calculate shortfall
//  3. Check if DistoVault can cover shortfall (loan)
//  4. Create loan record and transfer from DistoVault → StakeVault
//  5. Withdraw from StakeVault again (should succeed now)
//  6. Apply loan policy lever if configured
//
// Returns error if both vaults have insufficient funds.
func (vm *VaultManager) FundContract(contractID string, amountRT float64) error {
	startTime := time.Now()
	defer func() {
		if vm.metrics != nil {
			vm.metrics.FundingLatencySeconds.Observe(time.Since(startTime).Seconds())
		}
	}()

	vm.logger.Debug("funding contract",
		"contract_id", contractID,
		"amount_rt", amountRT)

	// Try StakeVault first (normal path)
	success, err := vm.vaultClient.WithdrawFromStakeVault(contractID, amountRT)
	if err != nil {
		return fmt.Errorf("withdraw from StakeVault failed: %w", err)
	}

	if success {
		// Funded from StakeVault - normal path (no loan needed)
		vm.logger.Info("contract funded from StakeVault",
			"contract_id", contractID,
			"amount_rt", amountRT)

		return nil
	}

	// Insufficient StakeVault - calculate shortfall and create loan
	stakeBalance, _ := vm.vaultClient.GetStakeVaultBalance()
	shortfall := amountRT - stakeBalance

	vm.logger.Debug("StakeVault insufficient, calculating loan",
		"contract_id", contractID,
		"requested_rt", amountRT,
		"stake_balance_rt", stakeBalance,
		"shortfall_rt", shortfall)

	// Check if DistoVault can cover the loan
	distoBalance, _ := vm.vaultClient.GetDistoVaultBalance()
	if distoBalance < shortfall {
		vm.logger.Warn("insufficient funds in both vaults",
			"contract_id", contractID,
			"requested_rt", amountRT,
			"stake_balance_rt", stakeBalance,
			"disto_balance_rt", distoBalance,
			"shortfall_rt", shortfall)

		return fmt.Errorf("insufficient funds: requested %.6f RT, StakeVault %.6f RT, DistoVault %.6f RT (shortfall %.6f RT)",
			amountRT, stakeBalance, distoBalance, shortfall)
	}

	// Create loan record
	loan := &Loan{
		LoanID:            uuid.New().String(),
		AmountRT:          shortfall,
		BorrowedAt:        time.Now(),
		ContractID:        contractID,
		Outstanding:       shortfall,
		Repaid:            0.0,
		TorqBumpRequested: vm.shouldRequestTorqBump(stakeBalance, distoBalance, shortfall),
	}

	// Transfer from DistoVault to StakeVault (temporary loan)
	success, err = vm.vaultClient.WithdrawFromDistoVault(loan.LoanID, shortfall)
	if err != nil {
		return fmt.Errorf("loan withdrawal from DistoVault failed: %w", err)
	}
	if !success {
		// Should never happen - we checked balance above
		return fmt.Errorf("loan withdrawal failed despite sufficient DistoVault balance")
	}

	// Deposit loan amount to StakeVault
	if err := vm.vaultClient.DepositToStakeVault(shortfall); err != nil {
		vm.logger.Error("CRITICAL: loan withdrawn from DistoVault but deposit to StakeVault failed",
			"loan_id", loan.LoanID,
			"shortfall_rt", shortfall,
			"error", err)
		return fmt.Errorf("loan deposit to StakeVault failed: %w", err)
	}

	// Record loan
	vm.addLoan(loan)

	// Now try to withdraw full amount from StakeVault (should succeed)
	success, err = vm.vaultClient.WithdrawFromStakeVault(contractID, amountRT)
	if err != nil {
		return fmt.Errorf("withdraw from StakeVault after loan failed: %w", err)
	}
	if !success {
		// Should never happen - we just deposited the shortfall
		return fmt.Errorf("withdraw from StakeVault failed after loan deposit")
	}

	vm.logger.Info("contract funded with loan",
		"contract_id", contractID,
		"amount_rt", amountRT,
		"loan_id", loan.LoanID,
		"loan_amount_rt", shortfall,
		"torq_bump_requested", loan.TorqBumpRequested)

	if vm.metrics != nil && loan.TorqBumpRequested {
		vm.metrics.TorqBumpRequestsTotal.Inc()
	}

	return nil
}

// RepayOutstandingLoans repays outstanding loans in FIFO order.
// Called after MintEvent deposits RoboStake to StakeVault.
//
// Algorithm:
//  1. For each loan in FIFO order:
//  2. Check StakeVault balance
//  3. Calculate repayment amount (min of outstanding, available balance)
//  4. Repay loan (withdraw from StakeVault, deposit to DistoVault)
//  5. Update loan outstanding balance
//  6. If loan fully repaid, remove from tracking
//  7. Continue until StakeVault depleted or all loans repaid
func (vm *VaultManager) RepayOutstandingLoans() error {
	vm.loansMu.Lock()
	defer vm.loansMu.Unlock()

	if len(vm.loans) == 0 {
		return nil // No loans to repay
	}

	vm.logger.Debug("repaying outstanding loans",
		"loan_count", len(vm.loans))

	// Track repayments for this cycle
	repaidLoans := make([]string, 0)

	// Process loans in FIFO order
	for _, loan := range vm.loans {
		if loan.Outstanding <= 0 {
			// Loan already fully repaid (shouldn't happen, but defensive)
			repaidLoans = append(repaidLoans, loan.LoanID)
			continue
		}

		// Check StakeVault balance
		stakeBalance, err := vm.vaultClient.GetStakeVaultBalance()
		if err != nil {
			return fmt.Errorf("get StakeVault balance failed: %w", err)
		}

		if stakeBalance <= 0.000001 { // Allow tiny rounding error
			// StakeVault depleted - stop repayment for now
			vm.logger.Debug("StakeVault depleted, stopping loan repayment",
				"remaining_loans", len(vm.loans)-len(repaidLoans))
			break
		}

		// Calculate repayment amount (min of outstanding, available balance)
		repaymentAmount := loan.Outstanding
		if stakeBalance < repaymentAmount {
			repaymentAmount = stakeBalance
		}

		// Repay loan (StakeVault → DistoVault)
		if err := vm.vaultClient.RepayLoan(loan.LoanID, repaymentAmount); err != nil {
			vm.logger.Error("loan repayment failed",
				"loan_id", loan.LoanID,
				"repayment_rt", repaymentAmount,
				"error", err)
			return fmt.Errorf("repay loan %s failed: %w", loan.LoanID, err)
		}

		// Update loan balance
		loan.Outstanding -= repaymentAmount
		loan.Repaid += repaymentAmount

		vm.logger.Debug("loan repayment",
			"loan_id", loan.LoanID,
			"repayment_rt", repaymentAmount,
			"outstanding_rt", loan.Outstanding,
			"total_repaid_rt", loan.Repaid)

		if vm.metrics != nil {
			vm.metrics.LoansPartialRepayments.Inc()
		}

		// Check if loan fully repaid
		if loan.Outstanding <= 0.000001 { // Allow tiny rounding error
			duration := time.Since(loan.BorrowedAt)
			vm.logger.Info("loan fully repaid",
				"loan_id", loan.LoanID,
				"contract_id", loan.ContractID,
				"amount_rt", loan.AmountRT,
				"duration_seconds", duration.Seconds())

			repaidLoans = append(repaidLoans, loan.LoanID)

			if vm.metrics != nil {
				vm.metrics.LoanDurationSeconds.Observe(duration.Seconds())
			}
		}
	}

	// Remove fully repaid loans
	for _, loanID := range repaidLoans {
		vm.removeLoan(loanID)
	}

	// Update loan metrics
	vm.updateLoanMetrics()

	vm.logger.Debug("loan repayment cycle complete",
		"repaid_count", len(repaidLoans),
		"remaining_loans", len(vm.loans))

	return nil
}

// GetVaultRatio returns the StakeVault / DistoVault ratio.
func (vm *VaultManager) GetVaultRatio() (float64, error) {
	stakeBalance, err := vm.vaultClient.GetStakeVaultBalance()
	if err != nil {
		return 0, err
	}

	distoBalance, err := vm.vaultClient.GetDistoVaultBalance()
	if err != nil {
		return 0, err
	}

	if distoBalance > 0 {
		return stakeBalance / distoBalance, nil
	} else if stakeBalance > 0 {
		return 999.999, nil // Arbitrarily large
	}
	return 0, nil // Both empty
}

// GetOutstandingLoans returns a copy of all outstanding loans.
func (vm *VaultManager) GetOutstandingLoans() []*Loan {
	vm.loansMu.RLock()
	defer vm.loansMu.RUnlock()

	// Return a copy to avoid external mutation
	loansCopy := make([]*Loan, len(vm.loans))
	copy(loansCopy, vm.loans)
	return loansCopy
}

// shouldRequestTorqBump determines if Torq bump should be requested based on loan policy.
func (vm *VaultManager) shouldRequestTorqBump(stakeBalance, distoBalance, loanAmount float64) bool {
	if !vm.config.TorqBumpEnabled {
		return false // Natural equilibrium (default)
	}

	switch vm.config.LoanPolicy {
	case config.LoanPolicyNatural:
		return false // Never request Torq bump

	case config.LoanPolicyTorqBumpImmediate:
		return true // Always request Torq bump on any loan

	case config.LoanPolicyTorqBumpThreshold:
		// Request Torq bump if StakeVault/DistoVault ratio below threshold
		var ratio float64
		if distoBalance > 0 {
			ratio = stakeBalance / distoBalance
		}
		return ratio < vm.config.TorqBumpThresholdRatio

	case config.LoanPolicyTorqBumpDelayed:
		// Note: Delay logic would be implemented in a background goroutine
		// For now, just return false (delay not yet implemented)
		return false

	default:
		vm.logger.Warn("unknown loan policy, defaulting to natural equilibrium",
			"policy", vm.config.LoanPolicy)
		return false
	}
}

// addLoan adds a loan to FIFO tracking.
func (vm *VaultManager) addLoan(loan *Loan) {
	vm.loansMu.Lock()
	defer vm.loansMu.Unlock()

	// Append to FIFO slice
	vm.loans = append(vm.loans, loan)
	vm.loanIndex[loan.LoanID] = len(vm.loans) - 1

	vm.logger.Debug("loan added to tracking",
		"loan_id", loan.LoanID,
		"amount_rt", loan.AmountRT,
		"total_loans", len(vm.loans))

	if vm.metrics != nil {
		vm.metrics.LoansCreatedTotal.Inc()
	}

	vm.updateLoanMetrics()
}

// removeLoan removes a fully repaid loan from tracking.
// Assumes loansMu is already locked.
func (vm *VaultManager) removeLoan(loanID string) {
	idx, exists := vm.loanIndex[loanID]
	if !exists {
		return
	}

	// Remove from slice (preserve FIFO order)
	vm.loans = append(vm.loans[:idx], vm.loans[idx+1:]...)

	// Rebuild index (indices shifted after removal)
	vm.loanIndex = make(map[string]int)
	for i, loan := range vm.loans {
		vm.loanIndex[loan.LoanID] = i
	}

	if vm.metrics != nil {
		vm.metrics.LoansRepaidTotal.Inc()
	}
}

// updateLoanMetrics updates loan-related Prometheus metrics.
// Assumes loansMu is already locked.
func (vm *VaultManager) updateLoanMetrics() {
	if vm.metrics == nil {
		return
	}

	// Count outstanding loans and total outstanding RT
	var totalOutstandingRT float64
	for _, loan := range vm.loans {
		totalOutstandingRT += loan.Outstanding
	}

	vm.metrics.LoansOutstandingCount.Set(float64(len(vm.loans)))
	vm.metrics.LoansOutstandingRT.Set(totalOutstandingRT)
}
