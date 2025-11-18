// contract_funder.go - Receives approved contracts from NATS and funds them via VaultManager
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/nats-io/nats.go"
)

// ContractFunder subscribes to contracts.approved and funds contracts via VaultManager
type ContractFunder struct {
	nc              *nats.Conn
	logger          *slog.Logger
	vaultManager    *VaultManager
	eventPublisher  *EventPublisher
	receiverMetrics *ReceiverMetrics
	vaultMetrics    *VaultMetrics
	topic           string
	damID           string // DistoDam instance identifier
	subscription    *nats.Subscription
	mu              sync.Mutex
	ctx             context.Context
	cancel          context.CancelFunc
}

// NewContractFunder creates a new ContractFunder
func NewContractFunder(
	nc *nats.Conn,
	logger *slog.Logger,
	vaultManager *VaultManager,
	eventPublisher *EventPublisher,
	receiverMetrics *ReceiverMetrics,
	vaultMetrics *VaultMetrics,
	topic string,
	damID string,
) *ContractFunder {
	ctx, cancel := context.WithCancel(context.Background())

	return &ContractFunder{
		nc:              nc,
		logger:          logger,
		vaultManager:    vaultManager,
		eventPublisher:  eventPublisher,
		receiverMetrics: receiverMetrics,
		vaultMetrics:    vaultMetrics,
		topic:           topic,
		damID:           damID,
		ctx:             ctx,
		cancel:          cancel,
	}
}

// Start begins listening for approved contracts on the configured topic
func (cf *ContractFunder) Start(ctx context.Context) error {
	cf.logger.Info("starting contract funder",
		"topic", cf.topic,
		"dam_id", cf.damID)

	// Subscribe to NATS topic
	sub, err := cf.nc.Subscribe(cf.topic, func(msg *nats.Msg) {
		cf.handleContract(msg)
	})

	if err != nil {
		cf.logger.Error("failed to subscribe to contracts",
			"topic", cf.topic,
			"error", err)
		return fmt.Errorf("subscribe to %s: %w", cf.topic, err)
	}

	cf.mu.Lock()
	cf.subscription = sub
	cf.mu.Unlock()

	pending, _, _ := sub.Pending()
	cf.logger.Info("contract funder started",
		"topic", cf.topic,
		"pending_messages", pending)

	// Block until context cancelled
	<-ctx.Done()

	cf.logger.Info("contract funder stopping due to context cancellation")
	return cf.Shutdown()
}

// handleContract processes a single approved contract message
func (cf *ContractFunder) handleContract(msg *nats.Msg) {
	startTime := time.Now()
	cf.receiverMetrics.ContractsReceivedTotal.Inc()

	cf.logger.Debug("received contract",
		"subject", msg.Subject,
		"size_bytes", len(msg.Data))

	// Parse Contract from JSON
	var contract Contract
	if err := json.Unmarshal(msg.Data, &contract); err != nil {
		cf.logger.Error("failed to parse contract",
			"error", err,
			"data_preview", string(msg.Data[:min(100, len(msg.Data))]))
		cf.receiverMetrics.ContractParseErrorsTotal.Inc()

		// NACK message so it can be redelivered
		msg.Nak()
		return
	}

	// Validate contract
	if err := ValidateContract(&contract); err != nil {
		cf.logger.Error("invalid contract",
			"error", err,
			"contract_id", contract.ID)
		cf.receiverMetrics.ContractValidationErrorsTotal.Inc()

		// ACK anyway - invalid contracts shouldn't be redelivered
		msg.Ack()
		return
	}

	cf.logger.Info("processing contract funding request",
		"contract_id", contract.ID,
		"robo_stake_rt", contract.RoboStake,
		"builder", contract.Builder,
		"roi", contract.ROI)

	// Fund contract via VaultManager
	// Note: VaultManager.FundContract handles both StakeVault and loan creation internally
	err := cf.vaultManager.FundContract(contract.ID, contract.RoboStake)
	if err != nil {
		cf.logger.Error("failed to fund contract",
			"contract_id", contract.ID,
			"robo_stake_rt", contract.RoboStake,
			"error", err)
		cf.receiverMetrics.ContractFundingErrorsTotal.Inc()

		// Check if insufficient funds (not a retryable error)
		if err.Error() == "insufficient funds in both vaults" {
			cf.logger.Warn("contract rejected due to insufficient funds",
				"contract_id", contract.ID,
				"robo_stake_rt", contract.RoboStake)
			cf.receiverMetrics.ContractsRejectedInsufficientFunds.Inc()

			// ACK message - insufficient funds isn't a bug, it's a policy decision
			// Trust service will timeout and retry later
			msg.Ack()
			return
		}

		// Other errors might be transient, NACK for retry
		msg.Nak()
		return
	}

	// Update contract with funding details
	contract.FundedAt = time.Now()
	contract.FundedBy = cf.damID
	contract.Status = "funded"

	// Check if loan was created (query VaultManager's loan tracking)
	// For Phase 6, we assume StakeVault funding (loan tracking is internal to VaultManager)
	contract.VaultSource = "stake_vault" // Simplified for Phase 6 MVP

	cf.logger.Info("contract funded successfully",
		"contract_id", contract.ID,
		"robo_stake_rt", contract.RoboStake,
		"vault_source", contract.VaultSource)

	// Publish ContractFundedEvent to NATS
	stakeBalance, _ := cf.vaultManager.vaultClient.GetStakeVaultBalance()
	distoBalance, _ := cf.vaultManager.vaultClient.GetDistoVaultBalance()
	vaultRatio, _ := cf.vaultManager.GetVaultRatio()

	fundedEvent := &ContractFundedEvent{
		EventID:    fmt.Sprintf("evt-contract-%s-%d", contract.ID, time.Now().UnixNano()),
		ContractID: contract.ID,
		AmountRT:   contract.RoboStake,
		Source:     contract.VaultSource,
		LoanID:     contract.LoanID,
		Timestamp:  contract.FundedAt,
		VaultRatio: vaultRatio,
	}

	if err := cf.eventPublisher.PublishContractFunded(cf.ctx, fundedEvent); err != nil {
		cf.logger.Error("failed to publish contract funded event",
			"contract_id", contract.ID,
			"event_id", fundedEvent.EventID,
			"error", err)
		// Don't NACK - funds already deducted, event publish failure is separate concern
		// EventPublisher has retry logic, will attempt to publish
	}

	// Update metrics
	cf.receiverMetrics.ContractsProcessedTotal.Inc()
	cf.receiverMetrics.ContractsFundedTotal.Inc()
	cf.receiverMetrics.ContractsFundedRTTotal.Add(contract.RoboStake)

	cf.vaultMetrics.ContractsFundedTotal.Inc()
	cf.vaultMetrics.ContractsFundedRoboTotal.Add(contract.RoboStake)

	duration := time.Since(startTime).Seconds()
	cf.vaultMetrics.FundingLatencySeconds.Observe(duration)

	cf.logger.Info("contract funded successfully",
		"contract_id", contract.ID,
		"robo_stake_rt", contract.RoboStake,
		"vault_source", contract.VaultSource,
		"stake_balance_rt", stakeBalance,
		"disto_balance_rt", distoBalance,
		"vault_ratio", vaultRatio,
		"processing_time_ms", duration*1000)

	// ACK message
	msg.Ack()
}

// Shutdown stops the ContractFunder and unsubscribes from NATS
func (cf *ContractFunder) Shutdown() error {
	cf.logger.Info("shutting down contract funder")

	cf.mu.Lock()
	defer cf.mu.Unlock()

	if cf.subscription != nil {
		// Drain pending messages before unsubscribing
		pending, _, err := cf.subscription.Pending()
		if err == nil && pending > 0 {
			cf.logger.Info("draining pending contracts",
				"pending_count", pending)
		}

		if err := cf.subscription.Unsubscribe(); err != nil {
			cf.logger.Error("failed to unsubscribe from contracts",
				"error", err)
			return fmt.Errorf("unsubscribe: %w", err)
		}

		cf.subscription = nil
	}

	cf.cancel()

	cf.logger.Info("contract funder shutdown complete")
	return nil
}
