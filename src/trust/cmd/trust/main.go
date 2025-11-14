package main

import (
	"context"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"b2b/natsx"
	"b2b/trust/internal/metrics"
	"b2b/trust/internal/trustsvc"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

func main() {
	logger, _ := zap.NewDevelopment()
	defer logger.Sync()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Connect to NATS
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = nats.DefaultURL // localhost:4222
	}
	
	natsClient, err := natsx.New(natsURL)
	if err != nil {
		logger.Fatal("Failed to connect to NATS", zap.String("url", natsURL), zap.Error(err))
	}
	defer natsClient.Close()
	logger.Info("Connected to NATS", zap.String("url", natsURL))

	// Metrics
	m := metrics.NewMetrics()

	// Trust service
	svc := trustsvc.NewService(logger, m, natsClient)
	mux := svc.Start(ctx)

	// Start HTTP server
	go func() {
		logger.Info("HTTP server listening on :8080")
		if err := http.ListenAndServe(":8080", mux); err != nil {
			logger.Fatal("HTTP server failed", zap.Error(err))
		}
	}()

	// Signal handling
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop

	logger.Info("Shutting down...")
	cancel()
	// Allow graceful shutdown
	time.Sleep(time.Second)
}
