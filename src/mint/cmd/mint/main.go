// Package main provides the entry point for the Mint service.
//
// The Mint service receives TokenTorqIngots from Refinery, aggregates them into batches,
// computes batch hashes, and publishes MintEvents to DistoDam via NATS.
//
// Architecture:
//
//	Refinery → IngotReceiver (HTTP) → IngotBuffer → BatchAggregator → MintEngine → DistoDamClient → NATS
//
// Graceful Shutdown:
//
//	SIGINT/SIGTERM → Cancel context → Flush remaining batch → Drain buffer → Close NATS → Stop HTTP
package main

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"
	"time"

	"b2b/mint/internal/config"
	"b2b/mint/internal/mint"

	"net/http"

	"github.com/prometheus/client_golang/prometheus/promhttp"
)

const (
	// ShutdownTimeout is the maximum time to wait for graceful shutdown
	ShutdownTimeout = 10 * time.Second
)

func main() {
	// Load configuration
	cfg := config.MustLoad()

	// Setup logger
	logger := setupLogger(cfg.LogLevel)
	logger.Info("Starting Mint service", "version", "0.1.0")
	logger.Info(cfg.String())

	// Create root context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Setup signal handling
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// Initialize components
	components, err := initializeComponents(ctx, cfg, logger)
	if err != nil {
		logger.Error("Failed to initialize components", "error", err)
		os.Exit(1)
	}

	// Start components
	errChan := make(chan error, 3)
	if err := startComponents(ctx, components, errChan); err != nil {
		logger.Error("Failed to start components", "error", err)
		shutdownComponents(components, logger)
		os.Exit(1)
	}

	logger.Info("Mint service started successfully",
		"http_port", cfg.HTTPPort,
		"nats_url", cfg.NatsURL,
	)

	// Wait for shutdown signal or error
	select {
	case sig := <-sigChan:
		logger.Info("Received shutdown signal", "signal", sig)
		cancel()
	case err := <-errChan:
		logger.Error("Component error", "error", err)
		cancel()
	}

	// Graceful shutdown
	logger.Info("Initiating graceful shutdown", "timeout", ShutdownTimeout)
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), ShutdownTimeout)
	defer shutdownCancel()

	if err := gracefulShutdown(shutdownCtx, components, logger); err != nil {
		logger.Error("Shutdown completed with errors", "error", err)
		os.Exit(1)
	}

	logger.Info("Mint service stopped successfully")
}

// Components holds all initialized service components
type Components struct {
	Buffer         mint.IngotBuffer
	Aggregator     mint.BatchAggregator
	Engine         mint.MintEngine
	Client         mint.DistoDamClient
	Receiver       mint.IngotReceiver
	Hasher         mint.BatchHasher
	Phase2Receiver *mint.Phase2IngotReceiver // Phase 3: Hash-only ingot receiver
}

// initializeComponents creates and initializes all service components
func initializeComponents(ctx context.Context, cfg *config.Config, logger *slog.Logger) (*Components, error) {
	logger.Info("Initializing components...")

	// Create IngotBuffer
	buffer := mint.NewIngotBuffer(cfg.BufferCapacity, logger)
	logger.Info("IngotBuffer initialized", "capacity", cfg.BufferCapacity)

	// Create DistoDamClient
	client := mint.NewDistoDamClient(cfg.NatsURL, logger)
	if err := client.Connect(); err != nil {
		return nil, fmt.Errorf("failed to connect to NATS: %w", err)
	}
	logger.Info("DistoDamClient connected", "nats_url", cfg.NatsURL)

	// Create BatchHasher
	hasher := mint.NewSimpleBatchHasher()
	logger.Info("SimpleBatchHasher initialized")

	// Create MintEngine
	engine := mint.NewMintEngine(hasher, client, logger)
	logger.Info("MintEngine initialized")

	// Create BatchAggregator
	aggregator := mint.NewBatchAggregator(
		buffer,
		engine,
		cfg.BatchSize,
		cfg.FlushInterval,
		logger,
	)
	logger.Info("BatchAggregator initialized",
		"batch_size", cfg.BatchSize,
		"flush_interval", cfg.FlushInterval,
	)

	// Create IngotReceiver (pass NATS connection for subscription)
	// The receiver will accept ingots from both HTTP and NATS
	receiver := mint.NewIngotReceiver(buffer, client.GetConnection(), cfg.HTTPPort, logger)
	logger.Info("IngotReceiver initialized", "http_port", cfg.HTTPPort, "nats_topic", "mint.ingots")

	// Create IngotHashQueue (Phase 3 Milestone 2: stores 1000 ingot hashes)
	ingotHashQueue, err := mint.NewIngotHashQueue(2000, 1000, logger)
	if err != nil {
		return nil, fmt.Errorf("failed to create IngotHashQueue: %w", err)
	}
	logger.Info("IngotHashQueue initialized",
		"capacity", 2000,
		"batch_size", 1000)

	// Create Phase2IngotReceiver (Phase 3: hash-only ingot receiver)
	phase2Receiver, err := mint.NewPhase2IngotReceiver(client.GetConnection(), ingotHashQueue, ctx, logger)
	if err != nil {
		return nil, fmt.Errorf("failed to create Phase2IngotReceiver: %w", err)
	}
	logger.Info("Phase2IngotReceiver initialized", "nats_topic", "mint.ingots")

	return &Components{
		Buffer:         buffer,
		Aggregator:     aggregator,
		Engine:         engine,
		Client:         client,
		Receiver:       receiver,
		Hasher:         hasher,
		Phase2Receiver: phase2Receiver,
	}, nil
}

// startComponents starts all service components in the correct order
func startComponents(ctx context.Context, components *Components, errChan chan error) error {
	// Start BatchAggregator (consumes from buffer, produces to engine)
	go func() {
		if err := components.Aggregator.Start(ctx); err != nil && err != context.Canceled {
			errChan <- fmt.Errorf("BatchAggregator error: %w", err)
		}
	}()

	// Start Prometheus metrics endpoint
	go func() {
		mux := http.NewServeMux()
		mux.Handle("/metrics", promhttp.Handler())
		server := &http.Server{
			Addr:    ":9090",
			Handler: mux,
		}
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errChan <- fmt.Errorf("Metrics server error: %w", err)
		}
	}()

	// Start IngotReceiver (HTTP server)
	go func() {
		if err := components.Receiver.Start(ctx); err != nil && err != http.ErrServerClosed {
			errChan <- fmt.Errorf("IngotReceiver error: %w", err)
		}
	}()

	// Start Phase2IngotReceiver (NATS subscriber for hash-only ingots)
	if err := components.Phase2Receiver.Start(); err != nil {
		return fmt.Errorf("failed to start Phase2IngotReceiver: %w", err)
	}

	// Give components time to start
	time.Sleep(100 * time.Millisecond)

	return nil
}

// gracefulShutdown performs coordinated shutdown of all components
func gracefulShutdown(ctx context.Context, components *Components, logger *slog.Logger) error {
	var shutdownErr error

	// Step 1: Stop accepting new ingots (shutdown HTTP server)
	logger.Info("Stopping IngotReceiver...")
	if err := components.Receiver.Shutdown(ctx); err != nil {
		logger.Error("Error shutting down IngotReceiver", "error", err)
		shutdownErr = err
	} else {
		logger.Info("IngotReceiver stopped")
	}

	// Step 1b: Stop Phase2IngotReceiver (NATS subscriber)
	logger.Info("Stopping Phase2IngotReceiver...")
	if err := components.Phase2Receiver.Stop(); err != nil {
		logger.Error("Error stopping Phase2IngotReceiver", "error", err)
		shutdownErr = err
	} else {
		logger.Info("Phase2IngotReceiver stopped")
	}

	// Step 2: Flush remaining batch (triggers MintEngine)
	logger.Info("Flushing remaining batch...")
	if err := components.Aggregator.Flush(); err != nil {
		logger.Error("Error flushing batch", "error", err)
		shutdownErr = err
	} else {
		accumulated := components.Aggregator.GetAccumulatedCount()
		logger.Info("Batch flushed", "ingots", accumulated)
	}

	// Step 3: Drain buffer of any remaining ingots
	logger.Info("Draining buffer...")
	remaining := components.Buffer.Drain()
	if len(remaining) > 0 {
		logger.Warn("Buffer had unprocessed ingots", "count", len(remaining))
		// Note: In production, might want to process these or save to disk
	} else {
		logger.Info("Buffer drained", "remaining", 0)
	}

	// Step 4: Close NATS connection (drain pending publishes)
	logger.Info("Closing NATS connection...")
	if err := components.Client.Close(); err != nil {
		logger.Error("Error closing NATS connection", "error", err)
		shutdownErr = err
	} else {
		logger.Info("NATS connection closed")
	}

	return shutdownErr
}

// shutdownComponents performs emergency shutdown (no graceful handling)
func shutdownComponents(components *Components, logger *slog.Logger) {
	logger.Warn("Performing emergency shutdown...")

	if components.Receiver != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		components.Receiver.Shutdown(ctx)
		cancel()
	}

	if components.Phase2Receiver != nil {
		components.Phase2Receiver.Stop()
	}

	if components.Client != nil {
		components.Client.Close()
	}

	logger.Info("Emergency shutdown complete")
}

// setupLogger creates a structured logger with the specified level
func setupLogger(levelStr string) *slog.Logger {
	var level slog.Level

	switch levelStr {
	case "debug":
		level = slog.LevelDebug
	case "info":
		level = slog.LevelInfo
	case "warn":
		level = slog.LevelWarn
	case "error":
		level = slog.LevelError
	default:
		level = slog.LevelInfo
	}

	opts := &slog.HandlerOptions{
		Level: level,
	}

	handler := slog.NewJSONHandler(os.Stdout, opts)
	return slog.New(handler)
}
