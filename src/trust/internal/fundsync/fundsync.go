package fundsync

import (
	"context"
	"encoding/json"

	"b2b/natsx"
	"b2b/trust/internal/contract"
	"b2b/trust/internal/metrics"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

// Start subscribes to NATS 'contracts.funded' topic and forwards funded contracts to executor
func Start(ctx context.Context, natsClient *natsx.Client, out chan<- *contract.Contract, logger *zap.Logger, m *metrics.Metrics) {
	// Subscribe to contracts.funded topic
	_, err := natsClient.Conn.Subscribe("contracts.funded", func(msg *nats.Msg) {
		// Parse the funded contract
		var c contract.Contract
		if err := json.Unmarshal(msg.Data, &c); err != nil {
			logger.Error("Failed to parse funded contract",
				zap.Error(err),
				zap.String("raw", string(msg.Data)))
			return
		}

		logger.Info("Received funded contract from DistoDam",
			zap.String("contract_id", c.ID),
			zap.String("opportunity_id", c.OpportunityID),
			zap.String("status", c.Status),
			zap.Float64("robo_stake", c.RoboStake))

		m.IncFundsSynced()
		m.IncContractsFunded()

		// Forward to executor
		select {
		case out <- &c:
			logger.Info("Forwarded funded contract to executor",
				zap.String("contract_id", c.ID))
		case <-ctx.Done():
			return
		default:
			logger.Warn("Executor channel full, dropping contract",
				zap.String("contract_id", c.ID))
		}
	})

	if err != nil {
		logger.Fatal("Failed to subscribe to contracts.funded",
			zap.Error(err))
	}

	logger.Info("FundSync subscribed to NATS topic", zap.String("topic", "contracts.funded"))

	// Block until context is done
	<-ctx.Done()
	logger.Info("FundSync shutting down")
}
