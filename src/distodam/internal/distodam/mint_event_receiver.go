// mint_event_receiver.go - Receives MintEvents from NATS and deposits RoboStake to StakeVault
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"

	"github.com/nats-io/nats.go"
)

// MintEventReceiver subscribes to distodam.units and processes Phase3RoboTorqUnit
type MintEventReceiver struct {
	nc               *nats.Conn
	logger           *slog.Logger
	vaultManager     *VaultManager
	certificateVault *CertificateVaultClient // Certificate-based vault
	metrics          *ReceiverMetrics
	topic            string
	subscription     *nats.Subscription
	mu               sync.Mutex
	ctx              context.Context
	cancel           context.CancelFunc
}

// NewMintEventReceiver creates a new MintEventReceiver
func NewMintEventReceiver(
	nc *nats.Conn,
	logger *slog.Logger,
	vaultManager *VaultManager,
	certificateVault *CertificateVaultClient,
	metrics *ReceiverMetrics,
	topic string,
) *MintEventReceiver {
	ctx, cancel := context.WithCancel(context.Background())

	return &MintEventReceiver{
		nc:               nc,
		logger:           logger,
		vaultManager:     vaultManager,
		certificateVault: certificateVault,
		metrics:          metrics,
		topic:            topic,
		ctx:              ctx,
		cancel:           cancel,
	}
}

// Start begins listening for MintEvents on the configured topic
func (mer *MintEventReceiver) Start(ctx context.Context) error {
	mer.logger.Info("starting mint event receiver",
		"topic", mer.topic)

	// Subscribe to NATS topic
	sub, err := mer.nc.Subscribe(mer.topic, func(msg *nats.Msg) {
		mer.handleMintEvent(msg)
	})

	if err != nil {
		mer.logger.Error("failed to subscribe to mint events",
			"topic", mer.topic,
			"error", err)
		return fmt.Errorf("subscribe to %s: %w", mer.topic, err)
	}

	mer.mu.Lock()
	mer.subscription = sub
	mer.mu.Unlock()

	pending, _, _ := sub.Pending()
	mer.logger.Info("mint event receiver started",
		"topic", mer.topic,
		"pending_messages", pending)

	// Block until context cancelled
	<-ctx.Done()

	mer.logger.Info("mint event receiver stopping due to context cancellation")
	return mer.Shutdown()
}

// handleMintEvent processes a single Phase3RoboTorqUnit message
func (mer *MintEventReceiver) handleMintEvent(msg *nats.Msg) {
	mer.metrics.MintEventsReceivedTotal.Inc()

	mer.logger.Debug("received Phase3 RT unit",
		"subject", msg.Subject,
		"size_bytes", len(msg.Data))

	// Parse Phase3RoboTorqUnit from JSON
	var unit Phase3RoboTorqUnit
	if err := json.Unmarshal(msg.Data, &unit); err != nil {
		mer.logger.Error("failed to parse Phase3 unit",
			"error", err,
			"data_preview", string(msg.Data[:min(100, len(msg.Data))]))
		mer.metrics.MintEventParseErrorsTotal.Inc()

		// NACK message so it can be redelivered
		msg.Nak()
		return
	}

	// Validate unit
	if unit.UnitID == "" || unit.RoboStakeTotal <= 0 {
		mer.logger.Error("invalid Phase3 unit",
			"unit_id", unit.UnitID,
			"robo_stake", unit.RoboStakeTotal)
		mer.metrics.MintEventValidationErrorsTotal.Inc()

		// ACK anyway - invalid units shouldn't be redelivered
		msg.Ack()
		return
	}

	mer.logger.Info("processing Phase3 RT unit",
		"unit_id", unit.UnitID,
		"robo_stake_rt", unit.RoboStakeTotal,
		"contracts", len(unit.ContractIDs),
		"merkle_root", unit.MerkleRoot)

	// Deposit RoboStake to StakeVault (economic cost returning)
	if err := mer.vaultManager.vaultClient.DepositToStakeVault(unit.RoboStakeTotal); err != nil {
		mer.logger.Error("failed to deposit RoboStake to StakeVault",
			"unit_id", unit.UnitID,
			"robo_stake_rt", unit.RoboStakeTotal,
			"error", err)
		mer.metrics.IngotStakeProcessingErrorsTotal.Inc()
		msg.Nak()
		return
	}

	stakeBalance, _ := mer.vaultManager.vaultClient.GetStakeVaultBalance()
	mer.logger.Info("RoboStake deposited to StakeVault",
		"unit_id", unit.UnitID,
		"robo_stake_deposited_rt", unit.RoboStakeTotal,
		"stake_vault_balance_rt", stakeBalance)

	// Deposit the actual Phase3RoboTorqUnit certificate to DistoVault
	// This is the CORRECT way - store the complete certificate!
	if err := mer.certificateVault.DepositToDistoVaultCertificate(&unit); err != nil {
		mer.logger.Error("failed to deposit RT certificate to DistoVault",
			"unit_id", unit.UnitID,
			"merkle_root", unit.MerkleRoot,
			"error", err)
		mer.metrics.IngotStakeProcessingErrorsTotal.Inc()
		msg.Nak()
		return
	}

	distoBalance, _ := mer.certificateVault.GetDistoVaultBalance()
	mer.logger.Info("Phase3 unit processed successfully",
		"unit_id", unit.UnitID,
		"robo_stake_deposited_rt", unit.RoboStakeTotal,
		"certificate_deposited", true,
		"merkle_root", unit.MerkleRoot,
		"stake_vault_balance_rt", stakeBalance,
		"disto_vault_certificates", distoBalance,
		"contracts", unit.ContractIDs)

	mer.metrics.MintEventsProcessedTotal.Inc()
	mer.metrics.IngotsProcessedTotal.Inc()
	mer.metrics.StakeDepositsTotal.Inc()
	mer.metrics.StakeDepositedRTTotal.Add(unit.RoboStakeTotal)

	// Update VaultMetrics inflow counters (for Grafana dashboard)
	if mer.vaultManager.metrics != nil {
		mer.vaultManager.metrics.InflowsTotal.Inc()
		mer.vaultManager.metrics.InflowsIngotStakesTotal.Inc()
		mer.vaultManager.metrics.InflowsRoboTotal.Add(unit.RoboStakeTotal)
	}

	// Trigger loan repayment after deposit (RoboStake has returned)
	mer.logger.Debug("triggering loan repayment",
		"deposited_rt", unit.RoboStakeTotal)
	mer.vaultManager.RepayOutstandingLoans()

	// ACK message
	msg.Ack()
}

// Shutdown stops the MintEventReceiver and unsubscribes from NATS
func (mer *MintEventReceiver) Shutdown() error {
	mer.logger.Info("shutting down mint event receiver")

	mer.mu.Lock()
	defer mer.mu.Unlock()

	if mer.subscription != nil {
		// Drain pending messages before unsubscribing
		pending, _, err := mer.subscription.Pending()
		if err == nil && pending > 0 {
			mer.logger.Info("draining pending mint events",
				"pending_count", pending)
		}

		if err := mer.subscription.Unsubscribe(); err != nil {
			mer.logger.Error("failed to unsubscribe from mint events",
				"error", err)
			return fmt.Errorf("unsubscribe: %w", err)
		}

		mer.subscription = nil
	}

	mer.cancel()

	mer.logger.Info("mint event receiver shutdown complete")
	return nil
}

// min helper function
func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
