package distodam

import (
	"fmt"
	"log/slog"
	"sync/atomic"
)

// MockVaultClient implements VaultClient using internal atomic.Int64 state.
// This is the Phase 6 MVP implementation - fast, simple, single-dam.
//
// In Phase 7+, replace with NATSVaultClient for distributed shadow vault architecture.
//
// Internal representation: micro-RT (1 RT = 1,000,000 µRT) for exact integer arithmetic.
type MockVaultClient struct {
	// Vault balances in micro-RT (atomic for thread-safety)
	stakeBalanceMicro atomic.Int64
	distoBalanceMicro atomic.Int64

	// Dependencies
	logger  *slog.Logger
	metrics *VaultMetrics
}

// NewMockVaultClient creates a new MockVaultClient with initial balances.
// Balances are specified in RT and converted to micro-RT internally.
func NewMockVaultClient(
	initialStakeRT float64,
	initialDistoRT float64,
	logger *slog.Logger,
	metrics *VaultMetrics,
) *MockVaultClient {
	mvc := &MockVaultClient{
		logger:  logger,
		metrics: metrics,
	}

	// Initialize balances (convert RT to micro-RT)
	mvc.stakeBalanceMicro.Store(rtToMicro(initialStakeRT))
	mvc.distoBalanceMicro.Store(rtToMicro(initialDistoRT))

	logger.Info("mock vault client initialized",
		"stake_balance_rt", initialStakeRT,
		"disto_balance_rt", initialDistoRT,
		"stake_balance_micro", mvc.stakeBalanceMicro.Load(),
		"disto_balance_micro", mvc.distoBalanceMicro.Load())

	// Update initial metrics
	if metrics != nil {
		metrics.StakeVaultBalanceRT.Set(initialStakeRT)
		metrics.StakeVaultBalanceMicro.Set(float64(mvc.stakeBalanceMicro.Load()))
		metrics.DistoVaultBalanceRT.Set(initialDistoRT)
		metrics.DistoVaultBalanceMicro.Set(float64(mvc.distoBalanceMicro.Load()))
		mvc.updateVaultRatio()
	}

	return mvc
}

// DepositToStakeVault deposits RoboTorq to the StakeVault.
func (m *MockVaultClient) DepositToStakeVault(amountRT float64) error {
	if amountRT < 0 {
		return fmt.Errorf("deposit amount must be non-negative, got %f RT", amountRT)
	}

	microRT := rtToMicro(amountRT)

	// Atomic add
	newBalance := m.stakeBalanceMicro.Add(microRT)

	m.logger.Debug("deposit to stake vault",
		"amount_rt", amountRT,
		"amount_micro", microRT,
		"new_balance_micro", newBalance,
		"new_balance_rt", microToRT(newBalance))

	// Update metrics
	if m.metrics != nil {
		m.metrics.StakeDepositsTotal.Inc()
		m.metrics.StakeDepositsRTTotal.Add(amountRT)
		m.metrics.StakeVaultBalanceRT.Set(microToRT(newBalance))
		m.metrics.StakeVaultBalanceMicro.Set(float64(newBalance))
		m.updateVaultRatio()
	}

	return nil
}

// DepositToDistoVault deposits newly minted RoboTorq to the DistoVault.
func (m *MockVaultClient) DepositToDistoVault(amountRT float64) error {
	if amountRT < 0 {
		return fmt.Errorf("deposit amount must be non-negative, got %f RT", amountRT)
	}

	microRT := rtToMicro(amountRT)

	// Atomic add
	newBalance := m.distoBalanceMicro.Add(microRT)

	m.logger.Debug("deposit to disto vault",
		"amount_rt", amountRT,
		"amount_micro", microRT,
		"new_balance_micro", newBalance,
		"new_balance_rt", microToRT(newBalance))

	// Update metrics
	if m.metrics != nil {
		m.metrics.DistoDepositsTotal.Inc()
		m.metrics.DistoDepositsRTTotal.Add(amountRT)
		m.metrics.DistoVaultBalanceRT.Set(microToRT(newBalance))
		m.metrics.DistoVaultBalanceMicro.Set(float64(newBalance))
		m.updateVaultRatio()
	}

	return nil
}

// WithdrawFromStakeVault attempts to withdraw RoboTorq from StakeVault for contract funding.
// Uses compare-and-swap loop for thread-safe all-or-nothing withdrawal.
func (m *MockVaultClient) WithdrawFromStakeVault(contractID string, amountRT float64) (bool, error) {
	if amountRT < 0 {
		return false, fmt.Errorf("withdrawal amount must be non-negative, got %f RT", amountRT)
	}

	microRT := rtToMicro(amountRT)

	// Compare-and-swap loop for atomic all-or-nothing withdrawal
	for {
		current := m.stakeBalanceMicro.Load()

		// Insufficient balance check
		if current < microRT {
			m.logger.Debug("stake vault insufficient balance",
				"contract_id", contractID,
				"requested_rt", amountRT,
				"requested_micro", microRT,
				"available_rt", microToRT(current),
				"available_micro", current)

			if m.metrics != nil {
				m.metrics.StakeWithdrawalsInsufficientBalance.Inc()
			}

			return false, nil // Not an error, triggers loan logic
		}

		// Attempt withdrawal
		newBalance := current - microRT
		if m.stakeBalanceMicro.CompareAndSwap(current, newBalance) {
			// Success!
			m.logger.Debug("withdraw from stake vault",
				"contract_id", contractID,
				"amount_rt", amountRT,
				"amount_micro", microRT,
				"new_balance_micro", newBalance,
				"new_balance_rt", microToRT(newBalance))

			// Update metrics
			if m.metrics != nil {
				m.metrics.StakeWithdrawalsTotal.Inc()
				m.metrics.StakeWithdrawalsRTTotal.Add(amountRT)
				m.metrics.StakeVaultBalanceRT.Set(microToRT(newBalance))
				m.metrics.StakeVaultBalanceMicro.Set(float64(newBalance))
				m.updateVaultRatio()
			}

			return true, nil
		}

		// CAS failed - retry (another goroutine modified balance)
	}
}

// WithdrawFromDistoVault attempts to withdraw RoboTorq from DistoVault for loan to StakeVault.
func (m *MockVaultClient) WithdrawFromDistoVault(loanID string, amountRT float64) (bool, error) {
	if amountRT < 0 {
		return false, fmt.Errorf("withdrawal amount must be non-negative, got %f RT", amountRT)
	}

	microRT := rtToMicro(amountRT)

	// Compare-and-swap loop
	for {
		current := m.distoBalanceMicro.Load()

		// Insufficient balance check
		if current < microRT {
			m.logger.Debug("disto vault insufficient balance",
				"loan_id", loanID,
				"requested_rt", amountRT,
				"requested_micro", microRT,
				"available_rt", microToRT(current),
				"available_micro", current)

			if m.metrics != nil {
				m.metrics.DistoWithdrawalsInsufficientBalance.Inc()
			}

			return false, nil // Not an error
		}

		// Attempt withdrawal
		newBalance := current - microRT
		if m.distoBalanceMicro.CompareAndSwap(current, newBalance) {
			m.logger.Debug("withdraw from disto vault",
				"loan_id", loanID,
				"amount_rt", amountRT,
				"amount_micro", microRT,
				"new_balance_micro", newBalance,
				"new_balance_rt", microToRT(newBalance))

			// Update metrics
			if m.metrics != nil {
				m.metrics.DistoWithdrawalsTotal.Inc()
				m.metrics.DistoWithdrawalsRTTotal.Add(amountRT)
				m.metrics.DistoVaultBalanceRT.Set(microToRT(newBalance))
				m.metrics.DistoVaultBalanceMicro.Set(float64(newBalance))
				m.updateVaultRatio()
			}

			return true, nil
		}

		// CAS failed - retry
	}
}

// GetStakeVaultBalance returns the current StakeVault balance in RT.
func (m *MockVaultClient) GetStakeVaultBalance() (float64, error) {
	microRT := m.stakeBalanceMicro.Load()
	return microToRT(microRT), nil
}

// GetDistoVaultBalance returns the current DistoVault balance in RT.
func (m *MockVaultClient) GetDistoVaultBalance() (float64, error) {
	microRT := m.distoBalanceMicro.Load()
	return microToRT(microRT), nil
}

// RepayLoan repays a loan from StakeVault back to DistoVault.
// This moves RT from StakeVault → DistoVault (reverse of loan).
func (m *MockVaultClient) RepayLoan(loanID string, amountRT float64) error {
	if amountRT < 0 {
		return fmt.Errorf("repayment amount must be non-negative, got %f RT", amountRT)
	}

	microRT := rtToMicro(amountRT)

	// Withdraw from StakeVault (must succeed - repayment assumes RoboStake has returned)
	success, err := m.WithdrawFromStakeVault(loanID+"-repay", amountRT)
	if err != nil {
		return fmt.Errorf("repay loan StakeVault withdrawal failed: %w", err)
	}
	if !success {
		return fmt.Errorf("repay loan insufficient StakeVault balance (expected RoboStake return)")
	}

	// Deposit to DistoVault
	if err := m.DepositToDistoVault(amountRT); err != nil {
		// Critical: We withdrew but failed to deposit - should never happen with atomic ops
		m.logger.Error("CRITICAL: loan repayment deposit failed after successful withdrawal",
			"loan_id", loanID,
			"amount_rt", amountRT,
			"error", err)
		return fmt.Errorf("repay loan DistoVault deposit failed: %w", err)
	}

	m.logger.Info("loan repaid",
		"loan_id", loanID,
		"amount_rt", amountRT,
		"amount_micro", microRT)

	// Update metrics
	if m.metrics != nil {
		m.metrics.LoansRepaidTotal.Inc()
	}

	return nil
}

// updateVaultRatio calculates and updates the StakeVault/DistoVault ratio metric.
func (m *MockVaultClient) updateVaultRatio() {
	if m.metrics == nil {
		return
	}

	stakeRT, _ := m.GetStakeVaultBalance()
	distoRT, _ := m.GetDistoVaultBalance()

	var ratio float64
	if distoRT > 0 {
		ratio = stakeRT / distoRT
	} else if stakeRT > 0 {
		ratio = 999.999 // Arbitrarily large (StakeVault has funds, DistoVault empty)
	} else {
		ratio = 0 // Both empty
	}

	m.metrics.VaultRatio.Set(ratio)
}

// Helper functions

// rtToMicro converts RT to micro-RT (1 RT = 1,000,000 µRT).
func rtToMicro(rt float64) int64 {
	return int64(rt * 1_000_000)
}

// microToRT converts micro-RT to RT.
func microToRT(micro int64) float64 {
	return float64(micro) / 1_000_000
}
