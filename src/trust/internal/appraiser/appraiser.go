package appraiser

import (
	"context"
	"fmt"
	"time"

	"b2b/trust/internal/contract"
	"b2b/trust/internal/metrics"
	"b2b/trust/internal/opportunity"

	"go.uber.org/zap"
)

const (
	// MinROI is the minimum ROI threshold for approving an opportunity
	MinROI = 0.10 // 10% minimum ROI
)

func Start(ctx context.Context, in <-chan *opportunity.Opportunity, out chan<- *contract.Contract, logger *zap.Logger, m *metrics.Metrics, workers int) {
	for i := 0; i < workers; i++ {
		go func(id int) {
			for {
				select {
				case opp := <-in:
					time.Sleep(100 * time.Millisecond) // simulate appraisal

					// Update opportunity status to appraising
					opp.UpdateStatus("appraising")

					// Extract ROI from the opportunity
					roi := opp.GetROI()
					diggerURL := opp.GetDiggerURL()

					logger.Info("Appraising opportunity",
						zap.String("opportunity", opp.ID),
						zap.Float64("roi", roi),
						zap.String("digger_url", diggerURL),
						zap.Int("worker", id))

					// ROI-based decision: approve if ROI >= MinROI
					if roi >= MinROI {
						// Approve the opportunity
						opp.UpdateStatus("approved")

						// Create a contract from the approved opportunity
						contractID := fmt.Sprintf("contract-%s", opp.ID)
						c := contract.New(
							contractID,
							opp.ID,
							opp.Builder,
							diggerURL,
							float64(opp.RequiredRT), // RoboStake amount
							roi,
							0, // Torq - will be set by DistoDam
						)

						logger.Info("Created contract from approved opportunity",
							zap.String("contract_id", contractID),
							zap.String("opportunity_id", opp.ID),
							zap.Float64("roi", roi),
							zap.Int("robo_stake", opp.RequiredRT))

						m.IncAppraised()
						m.IncContractsCreated()

						// Send contract to next stage (will be NATS publishing in todo #4)
						select {
						case out <- c:
						default:
							logger.Warn("Contract channel full, dropping contract",
								zap.String("contract_id", contractID))
						}
					} else {
						// Reject the opportunity
						opp.UpdateStatus("rejected")
						logger.Info("Rejected opportunity - ROI below threshold",
							zap.String("opportunity", opp.ID),
							zap.Float64("roi", roi),
							zap.Float64("min_roi", MinROI))
						m.IncAppraised()
					}

				case <-ctx.Done():
					return
				}
			}
		}(i)
	}
}
