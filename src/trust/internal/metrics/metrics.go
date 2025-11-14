package metrics

import (
	"context"
	"sync/atomic"
	"time"

	"go.uber.org/zap"
)

type Metrics struct {
	Submitted   uint64
	Appraised   uint64
	FundsSynced uint64
	Executions  uint64
}

func NewMetrics() *Metrics {
	return &Metrics{}
}

func (m *Metrics) IncSubmitted() {
	atomic.AddUint64(&m.Submitted, 1)
}

func (m *Metrics) IncAppraised() {
	atomic.AddUint64(&m.Appraised, 1)
}

func (m *Metrics) IncFundsSynced() {
	atomic.AddUint64(&m.FundsSynced, 1)
}

func (m *Metrics) IncExecutions() {
	atomic.AddUint64(&m.Executions, 1)
}

// Getters for reading metric values atomically
func (m *Metrics) GetSubmitted() uint64 {
	return atomic.LoadUint64(&m.Submitted)
}

func (m *Metrics) GetAppraised() uint64 {
	return atomic.LoadUint64(&m.Appraised)
}

func (m *Metrics) GetFundsSynced() uint64 {
	return atomic.LoadUint64(&m.FundsSynced)
}

func (m *Metrics) GetExecutions() uint64 {
	return atomic.LoadUint64(&m.Executions)
}

func (m *Metrics) Report(logger *zap.Logger) {
	logger.Info("Metrics Report",
		zap.Uint64("Submitted", atomic.LoadUint64(&m.Submitted)),
		zap.Uint64("Appraised", atomic.LoadUint64(&m.Appraised)),
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
