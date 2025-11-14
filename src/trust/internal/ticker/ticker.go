package ticker

import (
	"context"
	"time"

	"b2b/trust/internal/metrics"

	"go.uber.org/zap"
)

func Start(ctx context.Context, ch chan<- string, logger *zap.Logger, m *metrics.Metrics) {
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		counter := 0
		for {
			select {
			case <-ticker.C:
				opportunity := "opportunity-" + time.Now().Format("150405") + "-" + string(counter)
				counter++
				select {
				case ch <- opportunity:
					logger.Info("Submitted opportunity", zap.String("opportunity", opportunity))
					m.IncSubmitted()
				default:
					logger.Warn("Submit channel full, dropping opportunity")
				}
			case <-ctx.Done():
				return
			}
		}
	}()
}
