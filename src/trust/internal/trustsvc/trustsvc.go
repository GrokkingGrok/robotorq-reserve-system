package trustsvc

import (
	"context"
	"log"
	"time"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/natsx"
)

type Service struct {
	nats    *natsx.Client
	metrics *metrics.MetricRegistry
	done    chan struct{}
}

func New(nc *natsx.Client, m *metrics.MetricRegistry) *Service {
	return &Service{
		nats:    nc,
		metrics: m,
		done:    make(chan struct{}),
	}
}

func (s *Service) Start(ctx context.Context) error {
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()

	log.Println("🚀 Trust service started.")
	for {
		select {
		case <-ctx.Done():
			close(s.done)
			return nil
		case <-ticker.C:
			log.Println("⏱️  periodic trust tick (placeholder)")
		}
	}
}

func (s *Service) Stop() {
	<-s.done
	log.Println("🧹 Trust service stopped.")
}
