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

	// Phase 2 Ingot Assembler: Builds merkle trees from 3600 hashes
	phase2Assembler := refinery.NewPhase2IngotAssembler(ctx, queueMgr, slog.Default())
	slog.Info("Phase 2 ingot assembler initialized (merkle tree builder)")

	// Phase 2 Batch Sender: Time-based batch publishing for Phase2 ingots
	phase2BatchSender := refinery.NewPhase2BatchSender(ctx, phase2Assembler, mintClient, cfg.IngotBatchInterval)
	slog.Info("Phase 2 batch sender initialized", "interval", cfg.IngotBatchInterval)

	// Hash Batch Receiver: Phase 5 - Falcon-1024 signature verification
	// SKIP_FALCON_VERIFICATION=true disables verification for testing
	skipFalconVerification := os.Getenv("SKIP_FALCON_VERIFICATION") == "true"
	hashBatchReceiver := refinery.NewHashBatchReceiver(queueMgr, skipFalconVerification)
	if skipFalconVerification {
		slog.Warn("⚠️  TESTING MODE: Falcon verification DISABLED")
	}
	slog.Info("✅ Phase 5: Hash batch receiver initialized with Falcon-1024 verification")

	// NATS Subscriber: Listens for hash batches from Diggers (Phase 2+)
	// Phase 5: Now includes Falcon verification via hashBatchReceiver
	natsSubscriber, err := refinery.NewNATSSubscriber(ctx, mintClient.Connection(), queueMgr, hashBatchReceiver)
	if err != nil {
		slog.Error("failed to create NATS subscriber", "error", err)
		os.Exit(1)
	}
	slog.Info("NATS subscriber initialized with Falcon verification", "subject", "ore.batch")

	// Ingot Archive: Stores completed ingots for querying
	// ingotArchive := refinery.NewIngotArchive(slog.Default())
	// slog.Info("ingot archive initialized")

	// Health Handler: Comprehensive status endpoint
	healthHandler := refinery.NewHealthHandler(queueMgr, mintClient, phase2Assembler)
	slog.Info("health handler initialized")

	// ─────────────────────────────────────────────────────────────
	// 4. Start Background Workers
	// ─────────────────────────────────────────────────────────────

	// Start Phase 2 Ingot Assembler (merkle tree builder)
	go func() {
		slog.Info("starting Phase 2 ingot assembler (merkle tree builder)...")
		phase2Assembler.Start()
	}()

	// Start Phase 2 Batch Sender (publishes Phase2 ingots to Mint)
	go func() {
		slog.Info("starting Phase 2 batch sender...")
		phase2BatchSender.Start()
	}()

	go func() {
		slog.Info("starting NATS subscriber...")
		natsSubscriber.Start()
	}()

	// ─────────────────────────────────────────────────────────────
	// 5. Register HTTP Routes
	// ─────────────────────────────────────────────────────────────
	mux := http.NewServeMux()
	mux.Handle("/metrics", promhttp.Handler())
	mux.HandleFunc("/health", healthHandler.HTTPHandler)

	slog.Info("http routes registered",
		"endpoints", []string{"/metrics", "/health"},
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

	// Stop NATS subscriber
	natsSubscriber.Stop()

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
