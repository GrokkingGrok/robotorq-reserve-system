// Package ticker auto-generates test opportunities for development and testing.
//
// It creates synthetic opportunities every second with incrementing ROI values
// to simulate a continuous stream of work. This allows testing the complete
// pipeline without external data sources.
//
// In production, this would be replaced by real opportunity discovery mechanisms.
package ticker

import (
	"context"
	"fmt"
	"time"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/opportunity"

	"go.uber.org/zap"
)

func Start(ctx context.Context, ch chan<- *opportunity.Opportunity, logger *zap.Logger, m *metrics.Metrics) {
	go func() {
		ticker := time.NewTicker(1 * time.Second)
		defer ticker.Stop()
		counter := 0
		for {
			select {
			case <-ticker.C:
				opp := opportunity.New(
					fmt.Sprintf("opportunity-%s-%d", time.Now().Format("150405"), counter),
					fmt.Sprintf("Builder-%d", counter),
					fmt.Sprintf("Description-%d", counter),
					"http://localhost:9000", // Default Digger URL for testing
					counter*100,
					0.15+float64(counter)*0.01, // ROI increases with each opportunity
				)
				counter++
				select {
				case ch <- opp:
					logger.Info("Submitted opportunity", zap.String("opportunity", opp.ID))
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
