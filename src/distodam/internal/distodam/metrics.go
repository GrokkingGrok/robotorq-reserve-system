package distodam

import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// VaultMetrics holds Prometheus metrics for dual-vault operations.
type VaultMetrics struct {
	// Vault state (dual vaults)
	StakeVaultBalanceRT    prometheus.Gauge
	StakeVaultBalanceMicro prometheus.Gauge
	DistoVaultBalanceRT    prometheus.Gauge
	DistoVaultBalanceMicro prometheus.Gauge
	VaultRatio             prometheus.Gauge // StakeVault / DistoVault

	// Vault deposits
	StakeDepositsTotal   prometheus.Counter
	StakeDepositsRTTotal prometheus.Counter
	DistoDepositsTotal   prometheus.Counter
	DistoDepositsRTTotal prometheus.Counter

	// Vault withdrawals
	StakeWithdrawalsTotal               prometheus.Counter
	StakeWithdrawalsRTTotal             prometheus.Counter
	StakeWithdrawalsInsufficientBalance prometheus.Counter
	DistoWithdrawalsTotal               prometheus.Counter
	DistoWithdrawalsRTTotal             prometheus.Counter
	DistoWithdrawalsInsufficientBalance prometheus.Counter

	// Loan tracking
	LoansCreatedTotal      prometheus.Counter
	LoansOutstandingCount  prometheus.Gauge
	LoansOutstandingRT     prometheus.Gauge
	LoansRepaidTotal       prometheus.Counter
	LoansPartialRepayments prometheus.Counter
	LoanDurationSeconds    prometheus.Histogram
	TorqBumpRequestsTotal  prometheus.Counter

	// Inflows (from Mint)
	InflowsTotal            prometheus.Counter
	InflowsIngotStakesTotal prometheus.Counter
	InflowsRoboTotal        prometheus.Counter
	InflowsInvalid          prometheus.Counter

	// Contract funding metrics moved to ReceiverMetrics
	// (ContractFunder uses ReceiverMetrics, not VaultMetrics)

	// UBD funding (future)
	UBDRequestsReceivedTotal     prometheus.Counter
	UBDFundedTotal               prometheus.Counter
	UBDFundedRoboTotal           prometheus.Counter
	UBDRejectedInsufficientFunds prometheus.Counter

	// Publishing
	PublishSuccessTotal  prometheus.Counter
	PublishFailuresTotal prometheus.Counter
	PublishRetryTotal    prometheus.Counter

	// Performance
	FundingLatencySeconds       prometheus.Histogram
	IngotStakeProcessingSeconds prometheus.Histogram
}

// NewVaultMetrics creates and registers all Prometheus metrics.
func NewVaultMetrics() *VaultMetrics {
	return &VaultMetrics{
		// Vault state
		StakeVaultBalanceRT: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_stake_vault_balance_rt",
			Help: "Current StakeVault balance in RoboTorq",
		}),
		StakeVaultBalanceMicro: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_stake_vault_balance_micro",
			Help: "Current StakeVault balance in micro-RT",
		}),
		DistoVaultBalanceRT: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_disto_vault_balance_rt",
			Help: "Current DistoVault balance in RoboTorq",
		}),
		DistoVaultBalanceMicro: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_disto_vault_balance_micro",
			Help: "Current DistoVault balance in micro-RT",
		}),
		VaultRatio: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_vault_ratio",
			Help: "Ratio of StakeVault / DistoVault balance",
		}),

		// Vault deposits
		StakeDepositsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_stake_deposits_total",
			Help: "Total number of deposits to StakeVault",
		}),
		StakeDepositsRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_stake_deposits_rt_total",
			Help: "Total RT deposited to StakeVault",
		}),
		DistoDepositsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_disto_deposits_total",
			Help: "Total number of deposits to DistoVault",
		}),
		DistoDepositsRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_disto_deposits_rt_total",
			Help: "Total RT deposited to DistoVault",
		}),

		// Vault withdrawals
		StakeWithdrawalsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_stake_withdrawals_total",
			Help: "Total number of successful withdrawals from StakeVault",
		}),
		StakeWithdrawalsRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_stake_withdrawals_rt_total",
			Help: "Total RT withdrawn from StakeVault",
		}),
		StakeWithdrawalsInsufficientBalance: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_stake_withdrawals_insufficient_balance_total",
			Help: "Total number of failed StakeVault withdrawals due to insufficient balance",
		}),
		DistoWithdrawalsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_disto_withdrawals_total",
			Help: "Total number of successful withdrawals from DistoVault",
		}),
		DistoWithdrawalsRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_disto_withdrawals_rt_total",
			Help: "Total RT withdrawn from DistoVault",
		}),
		DistoWithdrawalsInsufficientBalance: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_disto_withdrawals_insufficient_balance_total",
			Help: "Total number of failed DistoVault withdrawals due to insufficient balance",
		}),

		// Loan tracking
		LoansCreatedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_loans_created_total",
			Help: "Total number of loans created (DistoVault → StakeVault)",
		}),
		LoansOutstandingCount: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_loans_outstanding_count",
			Help: "Current number of outstanding loans",
		}),
		LoansOutstandingRT: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_loans_outstanding_rt",
			Help: "Total RT currently on loan",
		}),
		LoansRepaidTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_loans_repaid_total",
			Help: "Total number of loans fully repaid",
		}),
		LoansPartialRepayments: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_loans_partial_repayments_total",
			Help: "Total number of partial loan repayment events",
		}),
		LoanDurationSeconds: promauto.NewHistogram(prometheus.HistogramOpts{
			Name:    "distodam_loan_duration_seconds",
			Help:    "Time from loan creation to full repayment",
			Buckets: prometheus.ExponentialBuckets(60, 2, 10), // 1min to ~17 hours
		}),
		TorqBumpRequestsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_torq_bump_requests_total",
			Help: "Total number of Torq bump requests (policy lever triggered)",
		}),

		// Inflows
		InflowsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_inflows_total",
			Help: "Total number of MintEvents received",
		}),
		InflowsIngotStakesTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_inflows_ingot_stakes_total",
			Help: "Total number of individual ingot stakes processed",
		}),
		InflowsRoboTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_inflows_robo_total",
			Help: "Total RT received from Mint (sum of ingot stakes)",
		}),
		InflowsInvalid: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_inflows_invalid_total",
			Help: "Total number of invalid MintEvents (parse errors)",
		}),

		// Contract funding metrics removed - see ReceiverMetrics

		// UBD funding (future)
		UBDRequestsReceivedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_requests_received_total",
			Help: "Total number of UBD requests received",
		}),
		UBDFundedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_funded_total",
			Help: "Total number of UBD requests successfully funded",
		}),
		UBDFundedRoboTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_funded_robo_total",
			Help: "Total RT distributed via UBD",
		}),
		UBDRejectedInsufficientFunds: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_rejected_insufficient_funds_total",
			Help: "Total number of UBD requests rejected due to insufficient funds",
		}),

		// Publishing
		PublishSuccessTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_publish_success_total",
			Help: "Total number of successful NATS publishes",
		}),
		PublishFailuresTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_publish_failures_total",
			Help: "Total number of failed NATS publishes",
		}),
		PublishRetryTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_publish_retry_total",
			Help: "Total number of NATS publish retries",
		}),

		// Performance
		FundingLatencySeconds: promauto.NewHistogram(prometheus.HistogramOpts{
			Name:    "distodam_funding_latency_seconds",
			Help:    "Time from contract received to funded published",
			Buckets: prometheus.ExponentialBuckets(0.001, 2, 10), // 1ms to ~1s
		}),
		IngotStakeProcessingSeconds: promauto.NewHistogram(prometheus.HistogramOpts{
			Name:    "distodam_ingot_stake_processing_seconds",
			Help:    "Time to process a single ingot stake",
			Buckets: prometheus.ExponentialBuckets(0.0001, 2, 10), // 0.1ms to ~100ms
		}),
	}
}

// ReceiverMetrics holds Prometheus metrics for NATS event receivers
type ReceiverMetrics struct {
	// MintEventReceiver metrics
	MintEventsReceivedTotal         prometheus.Counter
	MintEventsProcessedTotal        prometheus.Counter
	MintEventParseErrorsTotal       prometheus.Counter
	MintEventValidationErrorsTotal  prometheus.Counter
	IngotsProcessedTotal            prometheus.Counter
	IngotStakeProcessingErrorsTotal prometheus.Counter
	StakeDepositsTotal              prometheus.Counter
	StakeDepositedRTTotal           prometheus.Counter

	// ContractFunder metrics
	ContractsReceivedTotal             prometheus.Counter
	ContractsProcessedTotal            prometheus.Counter
	ContractParseErrorsTotal           prometheus.Counter
	ContractValidationErrorsTotal      prometheus.Counter
	ContractsFundedTotal               prometheus.Counter
	ContractsFundedRTTotal             prometheus.Counter
	ContractsFundedWithLoanTotal       prometheus.Counter
	ContractsRejectedInsufficientFunds prometheus.Counter
	ContractFundingErrorsTotal         prometheus.Counter

	// UBDRegistryReceiver metrics (future)
	UBDWalletsRegisteredTotal prometheus.Counter
	UBDWalletsActiveTotal     prometheus.Gauge

	// UBD distribution metrics (future)
	UBDDistributionsTotal prometheus.Counter
	UBDDistributedRTTotal prometheus.Counter
}

// NewReceiverMetrics creates a new ReceiverMetrics instance
func NewReceiverMetrics() *ReceiverMetrics {
	return &ReceiverMetrics{
		// MintEventReceiver
		MintEventsReceivedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_mint_events_received_total",
			Help: "Total number of MintEvents received from NATS",
		}),
		MintEventsProcessedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_mint_events_processed_total",
			Help: "Total number of MintEvents successfully processed",
		}),
		MintEventParseErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_mint_event_parse_errors_total",
			Help: "Total number of MintEvent JSON parse errors",
		}),
		MintEventValidationErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_mint_event_validation_errors_total",
			Help: "Total number of MintEvent validation errors",
		}),
		IngotsProcessedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ingots_processed_total",
			Help: "Total number of ingot stakes processed",
		}),
		IngotStakeProcessingErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ingot_stake_processing_errors_total",
			Help: "Total number of ingot stake processing errors",
		}),
		StakeDepositsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_receiver_stake_deposits_total",
			Help: "Total number of StakeVault deposits from receivers",
		}),
		StakeDepositedRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_receiver_stake_deposited_rt_total",
			Help: "Total RT deposited to StakeVault from receivers",
		}),

		// ContractFunder
		ContractsReceivedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_received_total",
			Help: "Total number of contracts received from NATS",
		}),
		ContractsProcessedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_processed_total",
			Help: "Total number of contracts successfully processed",
		}),
		ContractParseErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contract_parse_errors_total",
			Help: "Total number of contract JSON parse errors",
		}),
		ContractValidationErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contract_validation_errors_total",
			Help: "Total number of contract validation errors",
		}),
		ContractsFundedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_total",
			Help: "Total number of contracts successfully funded",
		}),
		ContractsFundedRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_rt_total",
			Help: "Total RT allocated to funded contracts",
		}),
		ContractsFundedWithLoanTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_with_loan_total",
			Help: "Total number of contracts funded via DistoVault loan",
		}),
		ContractsRejectedInsufficientFunds: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_rejected_insufficient_funds_total",
			Help: "Total number of contracts rejected due to insufficient funds",
		}),
		ContractFundingErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contract_funding_errors_total",
			Help: "Total number of contract funding errors",
		}),

		// UBDRegistryReceiver (future)
		UBDWalletsRegisteredTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_wallets_registered_total",
			Help: "Total number of UBD wallets registered",
		}),
		UBDWalletsActiveTotal: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "distodam_ubd_wallets_active_total",
			Help: "Current number of active UBD wallets",
		}),

		// UBD distributions (future)
		UBDDistributionsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_distributions_total",
			Help: "Total number of UBD distributions made",
		}),
		UBDDistributedRTTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_ubd_distributed_rt_total",
			Help: "Total RT distributed as UBD",
		}),
	}
}
