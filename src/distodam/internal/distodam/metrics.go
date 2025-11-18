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

	// Contract funding
	ContractsReceivedTotal             prometheus.Counter
	ContractsFundedTotal               prometheus.Counter
	ContractsFundedRoboTotal           prometheus.Counter
	ContractsFundedWithLoan            prometheus.Counter
	ContractsRejectedInsufficientFunds prometheus.Counter

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

		// Contract funding
		ContractsReceivedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_received_total",
			Help: "Total number of approved contracts received",
		}),
		ContractsFundedTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_total",
			Help: "Total number of contracts successfully funded",
		}),
		ContractsFundedRoboTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_robo_total",
			Help: "Total RT funded to contracts",
		}),
		ContractsFundedWithLoan: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_funded_with_loan_total",
			Help: "Total number of contracts funded using DistoVault loans",
		}),
		ContractsRejectedInsufficientFunds: promauto.NewCounter(prometheus.CounterOpts{
			Name: "distodam_contracts_rejected_insufficient_funds_total",
			Help: "Total number of contracts rejected due to insufficient funds",
		}),

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
