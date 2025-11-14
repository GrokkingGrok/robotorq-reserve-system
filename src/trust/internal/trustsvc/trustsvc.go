package trustsvc

import (
	"context"
	"net/http"
	"time"

	"b2b/natsx"
	"b2b/trust/internal/appraiser"
	"b2b/trust/internal/contract"
	"b2b/trust/internal/executor"
	"b2b/trust/internal/fundsync"
	"b2b/trust/internal/httpapi"
	"b2b/trust/internal/metrics"
	"b2b/trust/internal/opportunity"
	"b2b/trust/internal/ticker"

	"go.uber.org/zap"
)

type Service struct {
	logger  *zap.Logger
	metrics *metrics.Metrics
	nats    *natsx.Client
}

func NewService(logger *zap.Logger, m *metrics.Metrics, nats *natsx.Client) *Service {
	return &Service{
		logger:  logger,
		metrics: m,
		nats:    nats,
	}
}
func (s *Service) Start(ctx context.Context) *http.ServeMux {
	// Channels
	submitCh := make(chan *opportunity.Opportunity, 10)
	contractCh := make(chan *contract.Contract, 10)      // Appraiser outputs contracts
	fundedContractCh := make(chan *contract.Contract, 10) // FundSync outputs funded contracts
	execCh := make(chan *contract.Contract, 10)          // Executor outputs executed contracts

	// Start metrics reporter
	metrics.StartReporter(s.logger, s.metrics, 10*time.Second, ctx)

	// Start ticker
	ticker.Start(ctx, submitCh, s.logger, s.metrics)

	// HTTP API
	api := httpapi.New(s.logger, s.metrics, submitCh)

	mux := http.NewServeMux()
	api.RegisterRoutes(mux)

	// Start pipeline workers
	appraiser.Start(ctx, submitCh, contractCh, s.logger, s.metrics, 2)
	
	// Contract publisher: consume contracts from appraiser and publish to NATS
	go func() {
		for {
			select {
			case c := <-contractCh:
				// Publish contract to NATS 'contracts.pending' topic
				if err := s.nats.PublishJSON("contracts.pending", c); err != nil {
					s.logger.Error("Failed to publish contract to NATS",
						zap.String("contract_id", c.ID),
						zap.String("subject", "contracts.pending"),
						zap.Error(err))
				} else {
					s.logger.Info("Published contract to NATS",
						zap.String("contract_id", c.ID),
						zap.String("opportunity_id", c.OpportunityID),
						zap.Float64("roi", c.ROI),
						zap.String("status", c.Status))
				}
			case <-ctx.Done():
				return
			}
		}
	}()
	
	// FundSync: Subscribe to NATS 'contracts.funded' and forward to executor
	go fundsync.Start(ctx, s.nats, fundedContractCh, s.logger, s.metrics)
	
	executor.Start(ctx, fundedContractCh, execCh, s.logger, s.metrics, 2)

	// Final execution log
	go func() {
		for {
			select {
			case c := <-execCh:
				s.logger.Info("Contract executed",
					zap.String("contract_id", c.ID),
					zap.String("opportunity_id", c.OpportunityID))
			case <-ctx.Done():
				return
			}
		}
	}()

	return mux
}
