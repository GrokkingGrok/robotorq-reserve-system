// certificate_vault.go - Stores actual Phase3RoboTorqUnit certificates (not floats!)
package distodam

import (
	"fmt"
	"log/slog"
	"sync"
)

// CertificateVault stores actual Phase3RoboTorqUnit certificates
// This is the CORRECT implementation - vaults hold certificates, not balances!
type CertificateVault struct {
	units  []*Phase3RoboTorqUnit
	mu     sync.RWMutex
	logger *slog.Logger
}

// NewCertificateVault creates a new certificate vault
func NewCertificateVault(logger *slog.Logger) *CertificateVault {
	return &CertificateVault{
		units:  make([]*Phase3RoboTorqUnit, 0),
		logger: logger,
	}
}

// Deposit stores a Phase3RoboTorqUnit certificate
func (cv *CertificateVault) Deposit(unit *Phase3RoboTorqUnit) error {
	cv.mu.Lock()
	defer cv.mu.Unlock()

	cv.units = append(cv.units, unit)

	cv.logger.Info("certificate deposited to vault",
		"unit_id", unit.UnitID,
		"merkle_root", unit.MerkleRoot,
		"total_certificates", len(cv.units))

	return nil
}

// Withdraw removes and returns a Phase3RoboTorqUnit certificate
// Returns nil if vault is empty
func (cv *CertificateVault) Withdraw() (*Phase3RoboTorqUnit, error) {
	cv.mu.Lock()
	defer cv.mu.Unlock()

	if len(cv.units) == 0 {
		return nil, nil // Empty vault (not an error)
	}

	// FIFO: Take first certificate
	unit := cv.units[0]
	cv.units = cv.units[1:]

	cv.logger.Info("certificate withdrawn from vault",
		"unit_id", unit.UnitID,
		"merkle_root", unit.MerkleRoot,
		"remaining_certificates", len(cv.units))

	return unit, nil
}

// GetBalance returns the number of certificates (1 certificate = 1 RT)
func (cv *CertificateVault) GetBalance() int {
	cv.mu.RLock()
	defer cv.mu.RUnlock()

	return len(cv.units)
}

// GetBalanceRT returns balance in RT (same as certificate count since 1 cert = 1 RT)
func (cv *CertificateVault) GetBalanceRT() float64 {
	cv.mu.RLock()
	defer cv.mu.RUnlock()

	return float64(len(cv.units))
}

// GetAllCertificates returns a copy of all certificates (for inspection)
func (cv *CertificateVault) GetAllCertificates() []*Phase3RoboTorqUnit {
	cv.mu.RLock()
	defer cv.mu.RUnlock()

	// Return a copy to prevent external mutation
	certs := make([]*Phase3RoboTorqUnit, len(cv.units))
	copy(certs, cv.units)
	return certs
}

// CertificateVaultClient implements VaultClient but stores actual certificates
type CertificateVaultClient struct {
	stakeVault *CertificateVault // Stores returning RoboStake (currently just balance tracking)
	distoVault *CertificateVault // Stores newly minted Phase3RoboTorqUnit certificates
	logger     *slog.Logger
	metrics    *VaultMetrics

	// StakeVault still uses float tracking (it receives RoboStake amounts, not units)
	stakeBalanceRT float64
	stakeMu        sync.RWMutex
}

// NewCertificateVaultClient creates a certificate-based vault client
func NewCertificateVaultClient(
	initialStakeRT float64,
	logger *slog.Logger,
	metrics *VaultMetrics,
) *CertificateVaultClient {
	return &CertificateVaultClient{
		stakeVault:     NewCertificateVault(logger),
		distoVault:     NewCertificateVault(logger),
		logger:         logger,
		metrics:        metrics,
		stakeBalanceRT: initialStakeRT,
	}
}

// DepositToStakeVault deposits RoboStake amount (not certificates)
// StakeVault tracks returning economic value as float
func (cvc *CertificateVaultClient) DepositToStakeVault(amountRT float64) error {
	if amountRT < 0 {
		return fmt.Errorf("deposit amount must be non-negative, got %f RT", amountRT)
	}

	cvc.stakeMu.Lock()
	cvc.stakeBalanceRT += amountRT
	newBalance := cvc.stakeBalanceRT
	cvc.stakeMu.Unlock()

	cvc.logger.Debug("deposit to stake vault",
		"amount_rt", amountRT,
		"new_balance_rt", newBalance)

	// Update metrics
	if cvc.metrics != nil {
		cvc.metrics.StakeDepositsTotal.Inc()
		cvc.metrics.StakeDepositsRTTotal.Add(amountRT)
		cvc.metrics.StakeVaultBalanceRT.Set(newBalance)
		cvc.updateVaultRatio()
	}

	return nil
}

// DepositToDistoVault deposits a Phase3RoboTorqUnit certificate
// This is the CORRECT way - DistoVault stores actual RT certificates!
func (cvc *CertificateVaultClient) DepositToDistoVaultCertificate(unit *Phase3RoboTorqUnit) error {
	if err := cvc.distoVault.Deposit(unit); err != nil {
		return fmt.Errorf("deposit certificate to DistoVault: %w", err)
	}

	// Update metrics
	if cvc.metrics != nil {
		cvc.metrics.DistoDepositsTotal.Inc()
		cvc.metrics.DistoDepositsRTTotal.Add(1.0) // 1 certificate = 1 RT
		cvc.metrics.DistoVaultBalanceRT.Set(cvc.distoVault.GetBalanceRT())
		cvc.updateVaultRatio()
	}

	return nil
}

// DepositToDistoVault is kept for interface compatibility but should use certificate version
func (cvc *CertificateVaultClient) DepositToDistoVault(amountRT float64) error {
	// This shouldn't be used anymore - DistoVault should receive certificates!
	cvc.logger.Warn("DepositToDistoVault called with float - should use DepositToDistoVaultCertificate",
		"amount_rt", amountRT)
	return fmt.Errorf("use DepositToDistoVaultCertificate for certificate storage")
}

// WithdrawFromStakeVault withdraws RoboStake amount for contract funding
func (cvc *CertificateVaultClient) WithdrawFromStakeVault(contractID string, amountRT float64) (bool, error) {
	if amountRT < 0 {
		return false, fmt.Errorf("withdrawal amount must be non-negative, got %f RT", amountRT)
	}

	cvc.stakeMu.Lock()
	defer cvc.stakeMu.Unlock()

	if cvc.stakeBalanceRT < amountRT {
		cvc.logger.Debug("stake vault insufficient balance",
			"contract_id", contractID,
			"requested_rt", amountRT,
			"available_rt", cvc.stakeBalanceRT)

		if cvc.metrics != nil {
			cvc.metrics.StakeWithdrawalsInsufficientBalance.Inc()
		}

		return false, nil
	}

	cvc.stakeBalanceRT -= amountRT

	cvc.logger.Debug("withdraw from stake vault",
		"contract_id", contractID,
		"amount_rt", amountRT,
		"new_balance_rt", cvc.stakeBalanceRT)

	// Update metrics
	if cvc.metrics != nil {
		cvc.metrics.StakeWithdrawalsTotal.Inc()
		cvc.metrics.StakeWithdrawalsRTTotal.Add(amountRT)
		cvc.metrics.StakeVaultBalanceRT.Set(cvc.stakeBalanceRT)
		cvc.updateVaultRatio()
	}

	return true, nil
}

// WithdrawFromDistoVault withdraws a Phase3RoboTorqUnit certificate (not a float!)
// Returns nil if vault is empty
func (cvc *CertificateVaultClient) WithdrawFromDistoVaultCertificate() (*Phase3RoboTorqUnit, error) {
	unit, err := cvc.distoVault.Withdraw()
	if err != nil {
		return nil, fmt.Errorf("withdraw certificate from DistoVault: %w", err)
	}

	if unit == nil {
		// Empty vault
		if cvc.metrics != nil {
			cvc.metrics.DistoWithdrawalsInsufficientBalance.Inc()
		}
		return nil, nil
	}

	// Update metrics
	if cvc.metrics != nil {
		cvc.metrics.DistoWithdrawalsTotal.Inc()
		cvc.metrics.DistoWithdrawalsRTTotal.Add(1.0) // 1 certificate = 1 RT
		cvc.metrics.DistoVaultBalanceRT.Set(cvc.distoVault.GetBalanceRT())
		cvc.updateVaultRatio()
	}

	return unit, nil
}

// WithdrawFromDistoVault kept for interface compatibility but returns error
func (cvc *CertificateVaultClient) WithdrawFromDistoVault(loanID string, amountRT float64) (bool, error) {
	cvc.logger.Warn("WithdrawFromDistoVault called with float - should use WithdrawFromDistoVaultCertificate",
		"loan_id", loanID,
		"amount_rt", amountRT)
	return false, fmt.Errorf("use WithdrawFromDistoVaultCertificate for certificate withdrawal")
}

// GetStakeVaultBalance returns StakeVault balance (float tracking)
func (cvc *CertificateVaultClient) GetStakeVaultBalance() (float64, error) {
	cvc.stakeMu.RLock()
	defer cvc.stakeMu.RUnlock()
	return cvc.stakeBalanceRT, nil
}

// GetDistoVaultBalance returns number of certificates (1 cert = 1 RT)
func (cvc *CertificateVaultClient) GetDistoVaultBalance() (float64, error) {
	return cvc.distoVault.GetBalanceRT(), nil
}

// RepayLoan repays from StakeVault to DistoVault
// Note: Loans are still float-based since they're temporary transfers
func (cvc *CertificateVaultClient) RepayLoan(loanID string, amountRT float64) error {
	if amountRT < 0 {
		return fmt.Errorf("repayment amount must be non-negative, got %f RT", amountRT)
	}

	// Withdraw from StakeVault
	success, err := cvc.WithdrawFromStakeVault(loanID+"-repay", amountRT)
	if err != nil {
		return fmt.Errorf("repay loan StakeVault withdrawal failed: %w", err)
	}
	if !success {
		return fmt.Errorf("repay loan insufficient StakeVault balance")
	}

	// For loan repayment, we just adjust the StakeVault balance
	// The actual DistoVault certificates remain unchanged
	cvc.logger.Info("loan repaid",
		"loan_id", loanID,
		"amount_rt", amountRT)

	if cvc.metrics != nil {
		cvc.metrics.LoansRepaidTotal.Inc()
	}

	return nil
}

// updateVaultRatio calculates and updates the StakeVault/DistoVault ratio metric
func (cvc *CertificateVaultClient) updateVaultRatio() {
	if cvc.metrics == nil {
		return
	}

	stakeRT, _ := cvc.GetStakeVaultBalance()
	distoRT, _ := cvc.GetDistoVaultBalance()

	var ratio float64
	if distoRT > 0 {
		ratio = stakeRT / distoRT
	} else if stakeRT > 0 {
		ratio = 999.999
	} else {
		ratio = 0
	}

	cvc.metrics.VaultRatio.Set(ratio)
}
