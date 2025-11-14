package metrics

import (
	"context"
	"sync/atomic"
	"time"

	"go.uber.org/zap"
)

type Metrics struct {
	OpportunitiesSubmitted uint64
	OpportunitiesAppraised uint64
	FundsSynced            uint64
	Executions             uint64
}

func NewMetrics() *Metrics {
	return &Metrics{}
}

func (m *Metrics) IncSubmitted() {
	atomic.AddUint64(&m.OpportunitiesSubmitted, 1)
}

func (m *Metrics) IncAppraised() {
	atomic.AddUint64(&m.OpportunitiesAppraised, 1)
}

func (m *Metrics) IncFundsSynced() {
	atomic.AddUint64(&m.FundsSynced, 1)
}

func (m *Metrics) IncExecutions() {
	atomic.AddUint64(&m.Executions, 1)
}

func (m *Metrics) Report(logger *zap.Logger) {
	logger.Info("Metrics Report",
		zap.Uint64("Submitted", atomic.LoadUint64(&m.OpportunitiesSubmitted)),
		zap.Uint64("Appraised", atomic.LoadUint64(&m.OpportunitiesAppraised)),
		zap.Uint64("FundsSynced", atomic.LoadUint64(&m.FundsSynced)),
		zap.Uint64("Executions", atomic.LoadUint64(&m.Executions)),
	)
}

func StartReporter(logger *zap.Logger, m *Metrics, interval time.Duration, ctx context.Context) {
	ticker := time.NewTicker(interval)
	go func() {
		for {
			select {
			case <-ticker.C:
				m.Report(logger)
			case <-ctx.Done():
				ticker.Stop()
				return
			}
		}
	}()
}
