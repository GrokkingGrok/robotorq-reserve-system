package fundsync

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
					time.Sleep(200 * time.Millisecond) // simulate fundsync
					logger.Info("Funds synced", zap.String("opportunity", opp), zap.Int("worker", id))
					m.IncFundsSynced()
					select {
					case out <- opp:
					default:
						logger.Warn("Funds channel full, dropping opportunity")
					}
				case <-ctx.Done():
					return
				}
			}
		}(i)
	}
}
