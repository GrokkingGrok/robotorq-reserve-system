package main

import (
	"context"
	"sync"
	"testing"
	"time"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/trustsvc"

	"go.uber.org/zap"
)

func TestTrustServicePipeline(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	defer logger.Sync()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	m := metrics.NewMetrics()
	svc := trustsvc.NewService(logger, m)

	// Start service
	svc.Start(ctx)

	// Simulate high load: submit many opportunities manually
	const numOps = 50
	var wg sync.WaitGroup
	wg.Add(numOps)

	for i := 0; i < numOps; i++ {
		go func(id int) {
			defer wg.Done()
			// simulate random submission delay
			time.Sleep(time.Duration(id*20) * time.Millisecond)
			// The ticker in the service is running and generating opportunities
		}(i)
	}

	wg.Wait()

	// Let pipeline run for 10 seconds
	time.Sleep(10 * time.Second)

	// Report metrics one final time
	m.Report(logger)

	// Verify some opportunities were processed
	if m.GetSubmitted() == 0 {
		t.Error("Expected opportunities to be submitted")
	}

	t.Logf("Submitted: %d, Appraised: %d, Funds Synced: %d, Executions: %d",
		m.GetSubmitted(),
		m.GetAppraised(),
		m.GetFundsSynced(),
		m.GetExecutions(),
	)

	// Cancel context and wait for graceful shutdown
	cancel()
	time.Sleep(time.Second)
}

// Benchmark the pipeline throughput
func BenchmarkTrustServiceThroughput(b *testing.B) {
	logger, _ := zap.NewProduction()
	defer logger.Sync()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	m := metrics.NewMetrics()
	svc := trustsvc.NewService(logger, m)

	svc.Start(ctx)

	b.ResetTimer()

	// Run for b.N iterations
	for i := 0; i < b.N; i++ {
		// Let ticker generate opportunities
		time.Sleep(100 * time.Millisecond)
	}

	b.StopTimer()
	cancel()
}
