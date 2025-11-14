package trustsvc

import (
	"context"
	"net/http"
	"time"

	"b2b/trust/internal/appraiser"
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
}

func NewService(logger *zap.Logger, m *metrics.Metrics) *Service {
	return &Service{
		logger:  logger,
		metrics: m,
	}
}
func (s *Service) Start(ctx context.Context) *http.ServeMux {
	// Channels
	submitCh := make(chan *opportunity.Opportunity, 10)
	appraisedCh := make(chan string, 10)
	fundsCh := make(chan string, 10)
	execCh := make(chan string, 10)

	// Start metrics reporter
	metrics.StartReporter(s.logger, s.metrics, 10*time.Second, ctx)

	// Start ticker
	ticker.Start(ctx, submitCh, s.logger, s.metrics)

	// HTTP API
	api := httpapi.New(s.logger, s.metrics, submitCh)

	mux := http.NewServeMux()
	api.RegisterRoutes(mux)

	// Start pipeline workers
	appraiser.Start(ctx, submitCh, appraisedCh, s.logger, s.metrics, 2)
	fundsync.Start(ctx, appraisedCh, fundsCh, s.logger, s.metrics, 2)
	executor.Start(ctx, fundsCh, execCh, s.logger, s.metrics, 2)

	// Final execution log
	go func() {
		for {
			select {
			case msg := <-execCh:
				s.logger.Info("Executed opportunity", zap.String("opportunity", msg))
				s.metrics.IncExecutions()
			case <-ctx.Done():
				return
			}
		}
	}()

	return mux
}
