// Package main implements the Refinery service - a modular system that receives
// JouleTorqOre from Diggers, assembles them into TokenTorqIngots, and publishes
// batches to the Mint service via NATS messaging.
//
// Architecture:
// - Config: Environment-based configuration
// - OreReceiver: HTTP endpoint for ore ingestion
// - QueueManager: Thread-safe buffering
// - IngotAssembler: 3600 JouleTorq threshold logic
// - BatchSender: Time-based batch collection
// - MintClient: NATS publishing with retry
// - HealthHandler: Comprehensive status endpoint
package main

import (
	"context"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/prometheus/client_golang/prometheus/promhttp"

	"b2b/refinery/internal/config"
	"b2b/refinery/internal/refinery"
)

func main() {
	// Configure structured JSON logging
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo})))
	slog.Info("Refinery service starting...")

	// ─────────────────────────────────────────────────────────────
	// 1. Load Configuration
	// ─────────────────────────────────────────────────────────────
	cfg := config.LoadConfig()
	slog.Info("configuration loaded",
		"nats_url", cfg.NatsURL,
		"batch_interval", cfg.IngotBatchInterval,
		"joule_queue_size", cfg.JouleQueueSize,
		"robo_queue_size", cfg.RoboQueueSize,
	)

	// ─────────────────────────────────────────────────────────────
	// 2. Create Context for Lifecycle Management
	// ─────────────────────────────────────────────────────────────
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// ─────────────────────────────────────────────────────────────
	// 3. Initialize Core Components
	// ─────────────────────────────────────────────────────────────

	// Queue Manager: Thread-safe buffering for JouleTorqUnits
	queueMgr := refinery.NewQueueManager(ctx, cfg.JouleQueueSize)
	slog.Info("queue manager initialized",
		"unit_capacity", cfg.JouleQueueSize,
	)

	// Mint Client: NATS connection with retry logic
	mintClient, err := refinery.NewMintClient(ctx, cfg)
	if err != nil {
		slog.Error("failed to create mint client", "error", err)
		os.Exit(1)
	}
	defer mintClient.Close()
	slog.Info("mint client connected", "nats_url", cfg.NatsURL)

	// Ingot Assembler: Accumulates joules to 3600 threshold
	assembler := refinery.NewIngotAssembler(ctx, queueMgr)
	slog.Info("ingot assembler initialized")

	// Batch Sender: Time-based batch publishing
	batchSender := refinery.NewBatchSender(ctx, assembler, mintClient, cfg.IngotBatchInterval)
	slog.Info("batch sender initialized", "interval", cfg.IngotBatchInterval)

	// Ore Receiver: HTTP handler for Digger submissions
	oreReceiver := refinery.NewOreReceiver(queueMgr)
	slog.Info("ore receiver initialized")

	// Health Handler: Comprehensive status endpoint
	healthHandler := refinery.NewHealthHandler(queueMgr, mintClient, assembler)
	slog.Info("health handler initialized")

	// ─────────────────────────────────────────────────────────────
	// 4. Start Background Workers
	// ─────────────────────────────────────────────────────────────
	go func() {
		slog.Info("starting ingot assembler...")
		assembler.Start()
	}()

	go func() {
		slog.Info("starting batch sender...")
		batchSender.Start()
	}()

	// ─────────────────────────────────────────────────────────────
	// 5. Register HTTP Routes
	// ─────────────────────────────────────────────────────────────
	mux := http.NewServeMux()
	mux.Handle("/metrics", promhttp.Handler())
	mux.HandleFunc("/health", healthHandler.HTTPHandler)
	mux.HandleFunc("/receive-ore", oreReceiver.HTTPHandler)

	slog.Info("http routes registered",
		"endpoints", []string{"/metrics", "/health", "/receive-ore"},
	)

	// ─────────────────────────────────────────────────────────────
	// 6. Start HTTP Server
	// ─────────────────────────────────────────────────────────────
	server := &http.Server{
		Addr:    ":8080",
		Handler: mux,
	}

	go func() {
		slog.Info("http server starting", "port", ":8080")
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("http server failed", "error", err)
			os.Exit(1)
		}
	}()

	// ─────────────────────────────────────────────────────────────
	// 7. Graceful Shutdown Handling
	// ─────────────────────────────────────────────────────────────
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	<-sigChan
	slog.Info("shutdown signal received, initiating graceful shutdown...")

	// Cancel context to stop all background workers
	cancel()

	// Shutdown HTTP server with timeout
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer shutdownCancel()

	if err := server.Shutdown(shutdownCtx); err != nil {
		slog.Error("http server forced to shutdown", "error", err)
	} else {
		slog.Info("http server stopped gracefully")
	}

	// Close queue manager
	queueMgr.Close()

	slog.Info("refinery service shutdown complete")
}
