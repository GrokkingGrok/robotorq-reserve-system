package executor

import (
	"context"
	"time"

	"b2b/trust/internal/metrics"

	"go.uber.org/zap"
)

func Start(ctx context.Context, in <-chan string, out chan<- string, logger *zap.Logger, m *metrics.Metrics, workers int) {
	for i := 0; i < workers; i++ {
		go func(id int) {
			for {
				select {
				case opp := <-in:
					time.Sleep(150 * time.Millisecond) // simulate execution
					logger.Info("Executing opportunity", zap.String("opportunity", opp), zap.Int("worker", id))
					m.IncExecutions()
					select {
					case out <- opp:
					default:
						logger.Warn("Exec channel full, dropping opportunity")
					}
				case <-ctx.Done():
					return
				}
			}
		}(i)
	}
}
