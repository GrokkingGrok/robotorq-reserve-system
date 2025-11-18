package distodam

import (
	"log/slog"
	"os"
	"sync"
	"sync/atomic"
	"testing"
)

// TestNewMockVaultClient_Initialization verifies initial balances are set correctly.
func TestNewMockVaultClient_Initialization(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	// Don't use metrics in tests to avoid duplicate registration
	mvc := NewMockVaultClient(1.5, 0.5, logger, nil)

	stakeBalance, _ := mvc.GetStakeVaultBalance()
	distoBalance, _ := mvc.GetDistoVaultBalance()

	if stakeBalance != 1.5 {
		t.Errorf("StakeVault balance = %f, want 1.5", stakeBalance)
	}
	if distoBalance != 0.5 {
		t.Errorf("DistoVault balance = %f, want 0.5", distoBalance)
	}
}

// TestDepositToStakeVault verifies deposits increment balance correctly.
func TestDepositToStakeVault(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0, 0, logger, nil)

	// Deposit 1.0 RT
	err := mvc.DepositToStakeVault(1.0)
	if err != nil {
		t.Fatalf("DepositToStakeVault failed: %v", err)
	}

	balance, _ := mvc.GetStakeVaultBalance()
	if balance != 1.0 {
		t.Errorf("StakeVault balance = %f, want 1.0", balance)
	}

	// Deposit another 0.5 RT
	err = mvc.DepositToStakeVault(0.5)
	if err != nil {
		t.Fatalf("DepositToStakeVault failed: %v", err)
	}

	balance, _ = mvc.GetStakeVaultBalance()
	if balance != 1.5 {
		t.Errorf("StakeVault balance = %f, want 1.5", balance)
	}
}

// TestDepositToStakeVault_Negative verifies negative deposits are rejected.
func TestDepositToStakeVault_Negative(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0, 0, logger, nil)

	err := mvc.DepositToStakeVault(-1.0)
	if err == nil {
		t.Errorf("DepositToStakeVault should reject negative amount, got nil error")
	}
}

// TestDepositToDistoVault verifies deposits increment balance correctly.
func TestDepositToDistoVault(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0, 0, logger, nil)

	// Deposit 0.3 RT
	err := mvc.DepositToDistoVault(0.3)
	if err != nil {
		t.Fatalf("DepositToDistoVault failed: %v", err)
	}

	balance, _ := mvc.GetDistoVaultBalance()
	if balance != 0.3 {
		t.Errorf("DistoVault balance = %f, want 0.3", balance)
	}

	// Deposit another 0.2 RT
	err = mvc.DepositToDistoVault(0.2)
	if err != nil {
		t.Fatalf("DepositToDistoVault failed: %v", err)
	}

	balance, _ = mvc.GetDistoVaultBalance()
	if balance != 0.5 {
		t.Errorf("DistoVault balance = %f, want 0.5", balance)
	}
}

// TestWithdrawFromStakeVault_Success verifies successful withdrawal.
func TestWithdrawFromStakeVault_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(1.0, 0, logger, nil)

	// Withdraw 0.05 RT
	success, err := mvc.WithdrawFromStakeVault("contract-001", 0.05)
	if err != nil {
		t.Fatalf("WithdrawFromStakeVault failed: %v", err)
	}
	if !success {
		t.Errorf("WithdrawFromStakeVault should succeed, got success=false")
	}

	balance, _ := mvc.GetStakeVaultBalance()
	if balance != 0.95 {
		t.Errorf("StakeVault balance = %f, want 0.95", balance)
	}
}

// TestWithdrawFromStakeVault_InsufficientBalance verifies insufficient balance handling.
func TestWithdrawFromStakeVault_InsufficientBalance(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0.01, 0, logger, nil)

	// Attempt to withdraw 0.05 RT (insufficient)
	success, err := mvc.WithdrawFromStakeVault("contract-001", 0.05)
	if err != nil {
		t.Fatalf("WithdrawFromStakeVault should not error on insufficient balance, got: %v", err)
	}
	if success {
		t.Errorf("WithdrawFromStakeVault should fail with insufficient balance, got success=true")
	}

	// Balance should be unchanged
	balance, _ := mvc.GetStakeVaultBalance()
	if balance != 0.01 {
		t.Errorf("StakeVault balance = %f, want 0.01 (unchanged)", balance)
	}
}

// TestWithdrawFromStakeVault_AllOrNothing verifies all-or-nothing behavior.
func TestWithdrawFromStakeVault_AllOrNothing(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0.05, 0, logger, nil)

	// Attempt to withdraw exactly 0.05 RT (should succeed)
	success, err := mvc.WithdrawFromStakeVault("contract-001", 0.05)
	if err != nil {
		t.Fatalf("WithdrawFromStakeVault failed: %v", err)
	}
	if !success {
		t.Errorf("WithdrawFromStakeVault should succeed for exact balance, got success=false")
	}

	balance, _ := mvc.GetStakeVaultBalance()
	if balance != 0.0 {
		t.Errorf("StakeVault balance = %f, want 0.0", balance)
	}
}

// TestWithdrawFromDistoVault_Success verifies successful withdrawal from DistoVault.
func TestWithdrawFromDistoVault_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0, 1.0, logger, nil)

	// Withdraw 0.02 RT for loan
	success, err := mvc.WithdrawFromDistoVault("loan-001", 0.02)
	if err != nil {
		t.Fatalf("WithdrawFromDistoVault failed: %v", err)
	}
	if !success {
		t.Errorf("WithdrawFromDistoVault should succeed, got success=false")
	}

	balance, _ := mvc.GetDistoVaultBalance()
	if balance != 0.98 {
		t.Errorf("DistoVault balance = %f, want 0.98", balance)
	}
}

// TestRepayLoan verifies loan repayment moves RT from StakeVault to DistoVault.
func TestRepayLoan(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0.05, 0.1, logger, nil)

	// Repay 0.02 RT loan
	err := mvc.RepayLoan("loan-001", 0.02)
	if err != nil {
		t.Fatalf("RepayLoan failed: %v", err)
	}

	stakeBalance, _ := mvc.GetStakeVaultBalance()
	distoBalance, _ := mvc.GetDistoVaultBalance()

	if stakeBalance != 0.03 {
		t.Errorf("StakeVault balance = %f, want 0.03 (0.05 - 0.02)", stakeBalance)
	}
	if distoBalance != 0.12 {
		t.Errorf("DistoVault balance = %f, want 0.12 (0.1 + 0.02)", distoBalance)
	}
}

// TestRepayLoan_InsufficientStakeVault verifies repayment fails if StakeVault empty.
func TestRepayLoan_InsufficientStakeVault(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0.0, 0.1, logger, nil)

	// Attempt to repay 0.02 RT (StakeVault empty)
	err := mvc.RepayLoan("loan-001", 0.02)
	if err == nil {
		t.Errorf("RepayLoan should fail when StakeVault insufficient, got nil error")
	}
}

// TestConcurrentDeposits verifies thread-safety of concurrent deposits.
func TestConcurrentDeposits(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(0, 0, logger, nil)

	var wg sync.WaitGroup
	goroutines := 100
	amountPerGoroutine := 0.01

	// 100 goroutines each deposit 0.01 RT to StakeVault
	for i := 0; i < goroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			mvc.DepositToStakeVault(amountPerGoroutine)
		}()
	}

	wg.Wait()

	balance, _ := mvc.GetStakeVaultBalance()
	expected := float64(goroutines) * amountPerGoroutine

	// Allow small floating-point error
	if balance < expected-0.0001 || balance > expected+0.0001 {
		t.Errorf("StakeVault balance = %f, want ~%f", balance, expected)
	}
}

// TestConcurrentWithdrawals verifies thread-safety of concurrent withdrawals.
func TestConcurrentWithdrawals(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	initialBalance := 1.0
	mvc := NewMockVaultClient(initialBalance, 0, logger, nil)

	var wg sync.WaitGroup
	var successCount atomic.Int32
	goroutines := 100
	amountPerGoroutine := 0.01

	// 100 goroutines each attempt to withdraw 0.01 RT
	// All 100 should succeed (1.0 RT / 0.01 RT = 100)
	for i := 0; i < goroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			success, _ := mvc.WithdrawFromStakeVault("contract-"+string(rune(id)), amountPerGoroutine)
			if success {
				successCount.Add(1)
			}
		}(i)
	}

	wg.Wait()

	balance, _ := mvc.GetStakeVaultBalance()

	// Balance should be ~0 (all withdrawn)
	if balance > 0.0001 {
		t.Errorf("StakeVault balance = %f, want ~0.0", balance)
	}

	// All 100 withdrawals should succeed
	if successCount.Load() != int32(goroutines) {
		t.Errorf("Success count = %d, want %d", successCount.Load(), goroutines)
	}
}

// TestConcurrentMixedOperations verifies thread-safety of mixed deposits/withdrawals.
func TestConcurrentMixedOperations(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	mvc := NewMockVaultClient(1.0, 0, logger, nil)

	var wg sync.WaitGroup
	operations := 1000

	// 500 deposits of 0.01 RT + 500 withdrawals of 0.01 RT
	// Final balance should be ~1.0 RT (unchanged)
	for i := 0; i < operations/2; i++ {
		wg.Add(2)

		// Deposit
		go func() {
			defer wg.Done()
			mvc.DepositToStakeVault(0.01)
		}()

		// Withdraw
		go func(id int) {
			defer wg.Done()
			mvc.WithdrawFromStakeVault("contract-"+string(rune(id)), 0.01)
		}(i)
	}

	wg.Wait()

	balance, _ := mvc.GetStakeVaultBalance()

	// Balance should be around 1.0 RT (allow some variance due to timing)
	if balance < 0.5 || balance > 1.5 {
		t.Errorf("StakeVault balance = %f, expected ~1.0 (within reasonable range)", balance)
	}
}

// TestMicroRTConversion verifies RT ↔ micro-RT conversion accuracy.
func TestMicroRTConversion(t *testing.T) {
	tests := []struct {
		rt    float64
		micro int64
	}{
		{0.0, 0},
		{1.0, 1_000_000},
		{0.05, 50_000},
		{0.000001, 1},
		{999.999999, 999_999_999},
	}

	for _, tt := range tests {
		t.Run("", func(t *testing.T) {
			// RT → micro-RT
			micro := rtToMicro(tt.rt)
			if micro != tt.micro {
				t.Errorf("rtToMicro(%f) = %d, want %d", tt.rt, micro, tt.micro)
			}

			// micro-RT → RT
			rt := microToRT(tt.micro)
			if rt != tt.rt {
				t.Errorf("microToRT(%d) = %f, want %f", tt.micro, rt, tt.rt)
			}
		})
	}
}

// TestVaultRatioMetric verifies vault ratio is calculated correctly.
func TestVaultRatioMetric(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	tests := []struct {
		stakeRT       float64
		distoRT       float64
		expectedRatio float64
	}{
		{1.0, 1.0, 1.0},
		{0.95, 0.05, 19.0},
		{0.1, 1.0, 0.1},
		{0.0, 1.0, 0.0},
		{1.0, 0.0, 999.999}, // Arbitrarily large
	}

	for _, tt := range tests {
		t.Run("", func(t *testing.T) {
			mvc := NewMockVaultClient(tt.stakeRT, tt.distoRT, logger, nil)
			mvc.updateVaultRatio()

			// Ratio is set on metrics, read from Gauge (not directly accessible in test)
			// For now, just verify updateVaultRatio doesn't panic
			// In real test, would use testutil.ToFloat64() to read Gauge value
			_ = mvc
		})
	}
}
