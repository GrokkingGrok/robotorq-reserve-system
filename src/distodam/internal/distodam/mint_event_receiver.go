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

// MintEventReceiver subscribes to mint.batches and deposits ingot stakes to StakeVault
type MintEventReceiver struct {
	nc           *nats.Conn
	logger       *slog.Logger
	vaultManager *VaultManager
	metrics      *ReceiverMetrics
	topic        string
	subscription *nats.Subscription
	mu           sync.Mutex
	ctx          context.Context
	cancel       context.CancelFunc
}

// NewMintEventReceiver creates a new MintEventReceiver
func NewMintEventReceiver(
	nc *nats.Conn,
	logger *slog.Logger,
	vaultManager *VaultManager,
	metrics *ReceiverMetrics,
	topic string,
) *MintEventReceiver {
	ctx, cancel := context.WithCancel(context.Background())

	return &MintEventReceiver{
		nc:           nc,
		logger:       logger,
		vaultManager: vaultManager,
		metrics:      metrics,
		topic:        topic,
		ctx:          ctx,
		cancel:       cancel,
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

// handleMintEvent processes a single MintEvent message
func (mer *MintEventReceiver) handleMintEvent(msg *nats.Msg) {
	mer.metrics.MintEventsReceivedTotal.Inc()

	mer.logger.Debug("received mint event",
		"subject", msg.Subject,
		"size_bytes", len(msg.Data))

	// Parse MintEvent from JSON
	var event MintEvent
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		mer.logger.Error("failed to parse mint event",
			"error", err,
			"data_preview", string(msg.Data[:min(100, len(msg.Data))]))
		mer.metrics.MintEventParseErrorsTotal.Inc()

		// NACK message so it can be redelivered
		msg.Nak()
		return
	}

	// Validate event
	if err := ValidateMintEvent(&event); err != nil {
		mer.logger.Error("invalid mint event",
			"error", err,
			"batch_id", event.BatchID)
		mer.metrics.MintEventValidationErrorsTotal.Inc()

		// ACK anyway - invalid events shouldn't be redelivered
		msg.Ack()
		return
	}

	mer.logger.Info("processing mint event",
		"batch_id", event.BatchID,
		"ingots_count", len(event.IngotStakes),
		"batch_hash", event.BatchHash)

	// Process each ingot stake separately
	totalDeposited := 0.0
	for i, stake := range event.IngotStakes {
		if err := mer.processIngotStake(&stake); err != nil {
			mer.logger.Error("failed to process ingot stake",
				"batch_id", event.BatchID,
				"ingot_index", i,
				"ingot_id", stake.IngotID,
				"error", err)
			mer.metrics.IngotStakeProcessingErrorsTotal.Inc()
			// Continue processing other stakes despite failure
			continue
		}

		totalDeposited += stake.RoboStakeTotal
	}

	mer.logger.Info("mint event processed successfully",
		"batch_id", event.BatchID,
		"ingots_processed", len(event.IngotStakes),
		"total_deposited_rt", totalDeposited)

	mer.metrics.MintEventsProcessedTotal.Inc()
	mer.metrics.StakeDepositedRTTotal.Add(totalDeposited)

	// Trigger loan repayment after deposits (RoboStake has returned)
	if totalDeposited > 0 {
		mer.logger.Debug("triggering loan repayment",
			"deposited_rt", totalDeposited)
		mer.vaultManager.RepayOutstandingLoans()
	}

	// ACK message
	msg.Ack()
}

// processIngotStake deposits RoboStake from a single ingot to StakeVault
func (mer *MintEventReceiver) processIngotStake(stake *IngotStake) error {
	if stake.RoboStakeTotal <= 0 {
		mer.logger.Warn("skipping ingot stake with non-positive amount",
			"ingot_id", stake.IngotID,
			"robo_stake", stake.RoboStakeTotal)
		return fmt.Errorf("invalid robo_stake: %f", stake.RoboStakeTotal)
	}

	mer.logger.Debug("depositing ingot stake to StakeVault",
		"ingot_id", stake.IngotID,
		"robo_stake_rt", stake.RoboStakeTotal,
		"contracts", stake.ContractIDs)

	// Deposit to StakeVault via VaultClient
	err := mer.vaultManager.vaultClient.DepositToStakeVault(stake.RoboStakeTotal)
	if err != nil {
		mer.logger.Error("failed to deposit to StakeVault",
			"ingot_id", stake.IngotID,
			"robo_stake_rt", stake.RoboStakeTotal,
			"error", err)
		return fmt.Errorf("deposit to StakeVault: %w", err)
	}

	balance, _ := mer.vaultManager.vaultClient.GetStakeVaultBalance()
	mer.logger.Debug("ingot stake deposited successfully",
		"ingot_id", stake.IngotID,
		"robo_stake_rt", stake.RoboStakeTotal,
		"stake_vault_balance_rt", balance)

	mer.metrics.IngotsProcessedTotal.Inc()
	mer.metrics.StakeDepositsTotal.Inc()

	return nil
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
