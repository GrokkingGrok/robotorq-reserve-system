// vault_manager_bench_test.go - Benchmark tests for VaultManager performance
package distodam

import (
	"b2b/distorouter/internal/config"
	"log/slog"
	"os"
	"testing"
)

// BenchmarkVaultOperations tests raw vault operation throughput
func BenchmarkVaultOperations(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(0.0, 0.0, logger, vaultMetrics)

	b.ResetTimer()

	b.Run("StakeVault Deposits", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			vaultClient.DepositToStakeVault(0.05)
		}
	})

	b.Run("StakeVault Withdrawals", func(b *testing.B) {
		// Pre-fund vault
		vaultClient.DepositToStakeVault(float64(b.N) * 0.05)
		b.ResetTimer()

		for i := 0; i < b.N; i++ {
			vaultClient.WithdrawFromStakeVault("contract-1", 0.05)
		}
	})

	b.Run("DistoVault Deposits", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			vaultClient.DepositToDistoVault(0.05)
		}
	})

	b.Run("Vault Balance Queries", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			vaultClient.GetStakeVaultBalance()
			vaultClient.GetDistoVaultBalance()
		}
	})
}

// BenchmarkFundContract tests end-to-end contract funding throughput
func BenchmarkFundContract(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(0.0, 0.0, logger, vaultMetrics)

	cfg := &config.Config{
		TorqBumpEnabled: false, // Natural equilibrium
		LoanPolicy:      "natural",
	}

	vm := NewVaultManager(vaultClient, cfg, logger, vaultMetrics)

	b.ResetTimer()

	b.Run("FromStakeVault Only", func(b *testing.B) {
		// Pre-fund StakeVault with enough for all contracts
		vm.vaultClient.DepositToStakeVault(float64(b.N) * 0.05)
		b.ResetTimer()

		for i := 0; i < b.N; i++ {
			vm.FundContract("contract-1", 0.05)
		}
	})

	b.Run("With Loan (90% stake, 10% loan)", func(b *testing.B) {
		// Pre-fund: 90% in StakeVault, 100% in DistoVault (for loans)
		stakeFunding := float64(b.N) * 0.05 * 0.9
		distoFunding := float64(b.N) * 0.05

		vm.vaultClient.DepositToStakeVault(stakeFunding)
		vm.vaultClient.DepositToDistoVault(distoFunding)
		b.ResetTimer()

		for i := 0; i < b.N; i++ {
			vm.FundContract("contract-1", 0.05)
		}
	})

	b.Run("LoanRepayment", func(b *testing.B) {
		// Create loans first
		vm.vaultClient.DepositToDistoVault(float64(b.N) * 0.05)

		for i := 0; i < b.N; i++ {
			vm.FundContract("contract-1", 0.05) // Will create loans (StakeVault empty)
		}

		// Now benchmark repayment
		vm.vaultClient.DepositToStakeVault(float64(b.N) * 0.05)
		b.ResetTimer()

		for i := 0; i < b.N; i++ {
			vm.RepayOutstandingLoans()
		}
	})
}

// BenchmarkConcurrentFunding tests concurrent contract funding (race condition stress)
func BenchmarkConcurrentFunding(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(0.0, 0.0, logger, vaultMetrics)

	cfg := &config.Config{
		TorqBumpEnabled: false,
		LoanPolicy:      "natural",
	}

	vm := NewVaultManager(vaultClient, cfg, logger, vaultMetrics)

	// Pre-fund StakeVault
	vm.vaultClient.DepositToStakeVault(float64(b.N) * 0.05)

	b.ResetTimer()

	b.RunParallel(func(pb *testing.PB) {
		i := 0
		for pb.Next() {
			vm.FundContract("contract-1", 0.05)
			i++
		}
	})
}

// BenchmarkLoanLifecycle tests complete loan creation → repayment cycle
func BenchmarkLoanLifecycle(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(0.0, 0.0, logger, vaultMetrics)

	cfg := &config.Config{
		TorqBumpEnabled: false,
		LoanPolicy:      "natural",
	}

	vm := NewVaultManager(vaultClient, cfg, logger, vaultMetrics)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		// 1. Fund DistoVault only (force loan creation)
		vm.vaultClient.DepositToDistoVault(0.05)

		// 2. Fund contract (creates loan)
		vm.FundContract("contract-1", 0.05)

		// 3. RoboStake returns to StakeVault
		vm.vaultClient.DepositToStakeVault(0.05)

		// 4. Repay loan
		vm.RepayOutstandingLoans()

		// Verify loan cleared
		if len(vm.GetOutstandingLoans()) != 0 {
			b.Fatalf("expected 0 outstanding loans, got %d", len(vm.GetOutstandingLoans()))
		}
	}
}

// BenchmarkMintEventProcessing tests ingot stake processing throughput
func BenchmarkMintEventProcessing(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(0.0, 0.0, logger, vaultMetrics)

	cfg := &config.Config{
		TorqBumpEnabled: false,
		LoanPolicy:      "natural",
	}

	vm := NewVaultManager(vaultClient, cfg, logger, vaultMetrics)

	// Sample MintEvent with 1000 ingot stakes
	event := &MintEvent{
		BatchID:   "bench-batch-1",
		BatchHash: "abcdef1234567890",
		IngotStakes: []IngotStake{
			{
				IngotID:        "ingot-1",
				RoboStakeTotal: 0.05,
				ContractIDs:    []string{"contract-1"},
			},
		},
	}

	b.ResetTimer()

	b.Run("Single Ingot Deposit", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			for _, stake := range event.IngotStakes {
				vm.vaultClient.DepositToStakeVault(stake.RoboStakeTotal)
			}
		}
	})

	b.Run("Batch 1000 Ingots", func(b *testing.B) {
		// Create event with 1000 ingots
		largeEvent := &MintEvent{
			BatchID:     "bench-batch-large",
			BatchHash:   "abcdef1234567890",
			IngotStakes: make([]IngotStake, 1000),
		}

		for i := 0; i < 1000; i++ {
			largeEvent.IngotStakes[i] = IngotStake{
				IngotID:        "ingot-" + string(rune(i)),
				RoboStakeTotal: 0.00005, // Small stake per ingot
				ContractIDs:    []string{"contract-1"},
			}
		}

		b.ResetTimer()

		for i := 0; i < b.N; i++ {
			for _, stake := range largeEvent.IngotStakes {
				vm.vaultClient.DepositToStakeVault(stake.RoboStakeTotal)
			}
		}
	})
}

// BenchmarkVaultRatioCalculation tests vault ratio metric computation
func BenchmarkVaultRatioCalculation(b *testing.B) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	vaultMetrics := NewVaultMetrics()
	vaultClient := NewMockVaultClient(10.0, 100.0, logger, vaultMetrics)

	cfg := &config.Config{
		TorqBumpEnabled: false,
		LoanPolicy:      "natural",
	}

	vm := NewVaultManager(vaultClient, cfg, logger, vaultMetrics)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		vm.GetVaultRatio()
	}
}
