package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"
	"time"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/trustsvc"

	"go.uber.org/zap"
)

func main() {
	logger, _ := zap.NewDevelopment()
	defer logger.Sync()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Metrics
	m := metrics.NewMetrics()

	// Trust service
	svc := trustsvc.NewService(logger, m)
	svc.Start(ctx)

	// Signal handling
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop

	logger.Info("Shutting down...")
	cancel()
	// Allow graceful shutdown
	time.Sleep(time.Second)
}
