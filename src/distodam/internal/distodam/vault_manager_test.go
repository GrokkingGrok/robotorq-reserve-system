package distodam

import (
	"log/slog"
	"os"
	"testing"
	"time"

	"b2b/distorouter/internal/config"
)

// TestNewVaultManager verifies VaultManager initialization.
func TestNewVaultManager(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(1.0, 0.5, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}

	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	if vm == nil {
		t.Fatal("NewVaultManager returned nil")
	}

	loans := vm.GetOutstandingLoans()
	if len(loans) != 0 {
		t.Errorf("Initial loans count = %d, want 0", len(loans))
	}
}

// TestFundContract_FromStakeVault verifies normal funding from StakeVault.
func TestFundContract_FromStakeVault(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(1.0, 0.5, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract for 0.05 RT (should come from StakeVault)
	err := vm.FundContract("contract-001", 0.05)
	if err != nil {
		t.Fatalf("FundContract failed: %v", err)
	}

	// Check StakeVault balance decreased
	stakeBalance, _ := vaultClient.GetStakeVaultBalance()
	if stakeBalance != 0.95 {
		t.Errorf("StakeVault balance = %f, want 0.95", stakeBalance)
	}

	// Check DistoVault unchanged
	distoBalance, _ := vaultClient.GetDistoVaultBalance()
	if distoBalance != 0.5 {
		t.Errorf("DistoVault balance = %f, want 0.5 (unchanged)", distoBalance)
	}

	// Check no loans created
	loans := vm.GetOutstandingLoans()
	if len(loans) != 0 {
		t.Errorf("Loans count = %d, want 0 (no loan needed)", len(loans))
	}
}

// TestFundContract_WithLoan verifies funding with loan from DistoVault.
func TestFundContract_WithLoan(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 1.0, logger, nil) // Small StakeVault
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract for 0.05 RT (needs loan)
	err := vm.FundContract("contract-001", 0.05)
	if err != nil {
		t.Fatalf("FundContract failed: %v", err)
	}

	// Check loan created
	loans := vm.GetOutstandingLoans()
	if len(loans) != 1 {
		t.Fatalf("Loans count = %d, want 1", len(loans))
	}

	loan := loans[0]
	expectedLoanAmount := 0.04 // 0.05 requested - 0.01 available = 0.04 shortfall
	if loan.AmountRT < expectedLoanAmount-0.0001 || loan.AmountRT > expectedLoanAmount+0.0001 {
		t.Errorf("Loan amount = %f, want ~%f", loan.AmountRT, expectedLoanAmount)
	}

	if loan.Outstanding != loan.AmountRT {
		t.Errorf("Loan outstanding = %f, want %f", loan.Outstanding, loan.AmountRT)
	}

	// Check StakeVault depleted
	stakeBalance, _ := vaultClient.GetStakeVaultBalance()
	if stakeBalance > 0.0001 {
		t.Errorf("StakeVault balance = %f, want ~0.0 (fully used)", stakeBalance)
	}

	// Check DistoVault decreased by loan amount
	distoBalance, _ := vaultClient.GetDistoVaultBalance()
	expectedDistoBalance := 1.0 - expectedLoanAmount
	if distoBalance < expectedDistoBalance-0.0001 || distoBalance > expectedDistoBalance+0.0001 {
		t.Errorf("DistoVault balance = %f, want ~%f", distoBalance, expectedDistoBalance)
	}
}

// TestFundContract_InsufficientFunds verifies rejection when both vaults depleted.
func TestFundContract_InsufficientFunds(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 0.02, logger, nil) // Both vaults low
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Attempt to fund contract for 0.05 RT (insufficient)
	err := vm.FundContract("contract-001", 0.05)
	if err == nil {
		t.Fatal("FundContract should fail with insufficient funds, got nil error")
	}

	// Check no loans created
	loans := vm.GetOutstandingLoans()
	if len(loans) != 0 {
		t.Errorf("Loans count = %d, want 0 (funding failed)", len(loans))
	}
}

// TestRepayOutstandingLoans_FullRepayment verifies full loan repayment.
func TestRepayOutstandingLoans_FullRepayment(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 1.0, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract (creates loan)
	vm.FundContract("contract-001", 0.05)

	// Verify loan created
	loans := vm.GetOutstandingLoans()
	if len(loans) != 1 {
		t.Fatalf("Loans count = %d, want 1", len(loans))
	}
	loanAmount := loans[0].AmountRT

	// Simulate RoboStake return (deposit to StakeVault)
	vaultClient.DepositToStakeVault(0.05)

	// Repay loans
	err := vm.RepayOutstandingLoans()
	if err != nil {
		t.Fatalf("RepayOutstandingLoans failed: %v", err)
	}

	// Check loan fully repaid and removed
	loans = vm.GetOutstandingLoans()
	if len(loans) != 0 {
		t.Errorf("Loans count = %d, want 0 (fully repaid)", len(loans))
	}

	// Check DistoVault increased (loan repaid)
	distoBalance, _ := vaultClient.GetDistoVaultBalance()
	expectedDistoBalance := 1.0 // Original 1.0 - loan + repayment = 1.0
	if distoBalance < expectedDistoBalance-0.0001 || distoBalance > expectedDistoBalance+0.0001 {
		t.Errorf("DistoVault balance = %f, want ~%f (loan repaid)", distoBalance, expectedDistoBalance)
	}

	// Check StakeVault decreased by loan repayment
	stakeBalance, _ := vaultClient.GetStakeVaultBalance()
	expectedStakeBalance := 0.05 - loanAmount // Deposited 0.05, repaid ~0.04
	if stakeBalance < expectedStakeBalance-0.0001 || stakeBalance > expectedStakeBalance+0.0001 {
		t.Errorf("StakeVault balance = %f, want ~%f", stakeBalance, expectedStakeBalance)
	}
}

// TestRepayOutstandingLoans_PartialRepayment verifies partial loan repayment.
func TestRepayOutstandingLoans_PartialRepayment(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 1.0, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract (creates loan of ~0.04 RT)
	vm.FundContract("contract-001", 0.05)

	loans := vm.GetOutstandingLoans()
	initialOutstanding := loans[0].Outstanding

	// Simulate partial RoboStake return (only 0.02 RT)
	vaultClient.DepositToStakeVault(0.02)

	// Repay loans (partial)
	err := vm.RepayOutstandingLoans()
	if err != nil {
		t.Fatalf("RepayOutstandingLoans failed: %v", err)
	}

	// Check loan still exists but outstanding decreased
	loans = vm.GetOutstandingLoans()
	if len(loans) != 1 {
		t.Fatalf("Loans count = %d, want 1 (partially repaid)", len(loans))
	}

	loan := loans[0]
	expectedOutstanding := initialOutstanding - 0.02
	if loan.Outstanding < expectedOutstanding-0.0001 || loan.Outstanding > expectedOutstanding+0.0001 {
		t.Errorf("Loan outstanding = %f, want ~%f (partial repayment)", loan.Outstanding, expectedOutstanding)
	}

	if loan.Repaid < 0.019 || loan.Repaid > 0.021 {
		t.Errorf("Loan repaid = %f, want ~0.02", loan.Repaid)
	}
}

// TestRepayOutstandingLoans_FIFOOrder verifies loans are repaid in FIFO order.
func TestRepayOutstandingLoans_FIFOOrder(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.001, 1.0, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Create 3 loans (in order)
	vm.FundContract("contract-001", 0.02) // Loan 1
	time.Sleep(1 * time.Millisecond)
	vm.FundContract("contract-002", 0.02) // Loan 2
	time.Sleep(1 * time.Millisecond)
	vm.FundContract("contract-003", 0.02) // Loan 3

	loans := vm.GetOutstandingLoans()
	if len(loans) != 3 {
		t.Fatalf("Loans count = %d, want 3", len(loans))
	}

	// Record loan IDs in order
	loan1ID := loans[0].LoanID
	loan2ID := loans[1].LoanID

	// Deposit enough to repay first loan only
	vaultClient.DepositToStakeVault(0.025)

	// Repay loans (FIFO - should repay loan 1 first)
	err := vm.RepayOutstandingLoans()
	if err != nil {
		t.Fatalf("RepayOutstandingLoans failed: %v", err)
	}

	loans = vm.GetOutstandingLoans()
	if len(loans) != 2 {
		t.Errorf("Loans count = %d, want 2 (first loan repaid)", len(loans))
	}

	// Check that loan1 was repaid (not in list)
	for _, loan := range loans {
		if loan.LoanID == loan1ID {
			t.Errorf("Loan 1 still exists, should be repaid (FIFO violation)")
		}
	}

	// Check that loan2 is first in queue now
	if len(loans) > 0 && loans[0].LoanID != loan2ID {
		t.Errorf("First loan ID = %s, want %s (FIFO order)", loans[0].LoanID, loan2ID)
	}
}

// TestTorqBumpPolicy_Natural verifies natural equilibrium (no Torq bump).
func TestTorqBumpPolicy_Natural(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 1.0, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract (creates loan)
	vm.FundContract("contract-001", 0.05)

	loans := vm.GetOutstandingLoans()
	if len(loans) != 1 {
		t.Fatalf("Loans count = %d, want 1", len(loans))
	}

	// Check Torq bump NOT requested (natural policy)
	if loans[0].TorqBumpRequested {
		t.Errorf("TorqBumpRequested = true, want false (natural policy)")
	}
}

// TestTorqBumpPolicy_Immediate verifies immediate Torq bump on any loan.
func TestTorqBumpPolicy_Immediate(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultClient := NewMockVaultClient(0.01, 1.0, logger, nil)
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyTorqBumpImmediate,
		TorqBumpEnabled: true,
	}
	vm := NewVaultManager(vaultClient, cfg, logger, nil)

	// Fund contract (creates loan)
	vm.FundContract("contract-001", 0.05)

	loans := vm.GetOutstandingLoans()
	if len(loans) != 1 {
		t.Fatalf("Loans count = %d, want 1", len(loans))
	}

	// Check Torq bump requested (immediate policy)
	if !loans[0].TorqBumpRequested {
		t.Errorf("TorqBumpRequested = false, want true (immediate policy)")
	}
}

// TestTorqBumpPolicy_Threshold verifies Torq bump when ratio below threshold.
func TestTorqBumpPolicy_Threshold(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	cfg := &config.Config{
		LoanPolicy:             config.LoanPolicyTorqBumpThreshold,
		TorqBumpEnabled:        true,
		TorqBumpThresholdRatio: 0.10, // StakeVault < 10% of DistoVault triggers bump
	}

	// Test 1: Ratio below threshold (0.01 / 1.0 = 0.01 < 0.10)
	vaultClient1 := NewMockVaultClient(0.01, 1.0, logger, nil)
	vm1 := NewVaultManager(vaultClient1, cfg, logger, nil)
	vm1.FundContract("contract-001", 0.05)

	loans1 := vm1.GetOutstandingLoans()
	if len(loans1) != 1 {
		t.Fatalf("Test 1: Loans count = %d, want 1", len(loans1))
	}
	if !loans1[0].TorqBumpRequested {
		t.Errorf("Test 1: TorqBumpRequested = false, want true (ratio below threshold)")
	}

	// Test 2: Ratio above threshold (0.50 / 1.0 = 0.50 > 0.10)
	vaultClient2 := NewMockVaultClient(0.50, 1.0, logger, nil)
	vm2 := NewVaultManager(vaultClient2, cfg, logger, nil)
	vm2.FundContract("contract-001", 0.60) // Creates loan

	loans2 := vm2.GetOutstandingLoans()
	if len(loans2) != 1 {
		t.Fatalf("Test 2: Loans count = %d, want 1", len(loans2))
	}
	if loans2[0].TorqBumpRequested {
		t.Errorf("Test 2: TorqBumpRequested = true, want false (ratio above threshold)")
	}
}

// TestGetVaultRatio verifies vault ratio calculation.
func TestGetVaultRatio(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	cfg := &config.Config{
		LoanPolicy:      config.LoanPolicyNatural,
		TorqBumpEnabled: false,
	}

	tests := []struct {
		name          string
		stakeBalance  float64
		distoBalance  float64
		expectedRatio float64
	}{
		{"equal", 1.0, 1.0, 1.0},
		{"stake_high", 0.95, 0.05, 19.0},
		{"stake_low", 0.05, 0.95, 0.0526},
		{"disto_zero", 1.0, 0.0, 999.999},
		{"both_zero", 0.0, 0.0, 0.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			vaultClient := NewMockVaultClient(tt.stakeBalance, tt.distoBalance, logger, nil)
			vm := NewVaultManager(vaultClient, cfg, logger, nil)

			ratio, err := vm.GetVaultRatio()
			if err != nil {
				t.Fatalf("GetVaultRatio failed: %v", err)
			}

			// Allow 1% error for floating point
			tolerance := tt.expectedRatio * 0.01
			if tolerance < 0.001 {
				tolerance = 0.001
			}

			if ratio < tt.expectedRatio-tolerance || ratio > tt.expectedRatio+tolerance {
				t.Errorf("Vault ratio = %f, want ~%f", ratio, tt.expectedRatio)
			}
		})
	}
}
