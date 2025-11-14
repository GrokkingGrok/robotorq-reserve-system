package appraiser

import (
	"context"
	"time"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/opportunity"

	"go.uber.org/zap"
)

func Start(ctx context.Context, in <-chan *opportunity.Opportunity, out chan<- string, logger *zap.Logger, m *metrics.Metrics, workers int) {
	for i := 0; i < workers; i++ {
		go func(id int) {
			for {
				select {
				case opp := <-in:
					time.Sleep(100 * time.Millisecond) // simulate appraisal
					logger.Info("Appraised opportunity", zap.String("opportunity", opp.ID), zap.Int("worker", id))
					m.IncAppraised()
					select {
					case out <- opp.ID:
					default:
						logger.Warn("Appraised channel full, dropping opportunity")
					}
				case <-ctx.Done():
					return
				}
			}
		}(i)
	}
}
