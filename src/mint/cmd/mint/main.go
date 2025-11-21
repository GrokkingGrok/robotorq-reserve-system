// Package main provides the entry point for the Mint service.
//
// The Mint service receives Phase2Ingots from Refinery via NATS, extracts hashes,
// builds merkle trees, assembles RoboTorqUnits, and publishes them to DistoDam.
//
// Phase 2/3 Architecture:
//
//	Refinery (NATS) → Phase2IngotReceiver → IngotHashQueue → Level2MerkleBuilder
//	                                                           ↓
//	                                   Phase3RoboTorqUnitAssembler → Phase3DistoDamPublisher (NATS)
//	                                   ↓
//	                                   VerificationHandler (HTTP API for proof queries)
//
// Graceful Shutdown:
//
//	SIGINT/SIGTERM → Cancel context → Drain hash queue → Finalize pending units → Close NATS connections
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

	"github.com/prometheus/client_golang/prometheus"
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

// Components holds all initialized service components (Phase 2/3 only)
type Components struct {
	Client              mint.DistoDamClient               // NATS connection to DistoDam
	Phase2Receiver      *mint.Phase2IngotReceiver         // Phase 2: Hash-only ingot receiver
	IngotHashQueue      *mint.IngotHashQueue              // Phase 2 Milestone 2: 1000 ingot hash queue
	Level2MerkleBuilder *mint.Level2MerkleBuilder         // Phase 2 Milestone 3: Merkle tree builder
	Phase3Assembler     *mint.Phase3RoboTorqUnitAssembler // Phase 2 Milestone 4: RT unit assembler
	Phase3Publisher     *mint.Phase3DistoDamPublisher     // Phase 2 Milestone 5: DistoDam publisher
	VerificationHandler *mint.VerificationHandler         // Phase 5: Merkle proof verification API
}

// initializeComponents creates and initializes all service components (Phase 2/3 only)
func initializeComponents(ctx context.Context, cfg *config.Config, logger *slog.Logger) (*Components, error) {
	logger.Info("Initializing components (Phase 2/3 architecture)...")

	// Create DistoDamClient (NATS connection)
	client := mint.NewDistoDamClient(cfg.NatsURL, logger)
	if err := client.Connect(); err != nil {
		return nil, fmt.Errorf("failed to connect to NATS: %w", err)
	}
	logger.Info("DistoDamClient connected", "nats_url", cfg.NatsURL)

	// Create IngotHashQueue (Phase 2 Milestone 2: stores 1000 ingot hashes)
	ingotHashQueue, err := mint.NewIngotHashQueue(2000, 1000, logger)
	if err != nil {
		return nil, fmt.Errorf("failed to create IngotHashQueue: %w", err)
	}
	logger.Info("IngotHashQueue initialized",
		"capacity", 2000,
		"batch_size", 1000)

	// Create Phase2IngotReceiver (Phase 2: hash-only ingot receiver)
	phase2Receiver, err := mint.NewPhase2IngotReceiver(client.GetConnection(), ingotHashQueue, ctx, logger)
	if err != nil {
		return nil, fmt.Errorf("failed to create Phase2IngotReceiver: %w", err)
	}
	logger.Info("Phase2IngotReceiver initialized", "nats_topic", "mint.phase2.ingots")

	// Create Level2MerkleBuilder (Phase 2 Milestone 3: builds merkle tree from 1000 ingot hashes)
	level2MerkleBuilder := mint.NewLevel2MerkleBuilder(ingotHashQueue, logger)
	logger.Info("Level2MerkleBuilder initialized", "batch_size", 1000)

	// Create Phase3AssemblerMetrics (Phase 2 Milestone 4b: Prometheus metrics)
	phase3Metrics := mint.NewPhase3AssemblerMetrics(prometheus.DefaultRegisterer)
	logger.Info("Phase3AssemblerMetrics initialized")

	// Create Phase3RoboTorqUnitAssembler (Phase 2 Milestone 4b: RT unit assembler)
	phase3Assembler, err := mint.NewPhase3RoboTorqUnitAssembler(
		logger,
		phase3Metrics,
		level2MerkleBuilder,
		10, // Channel capacity
	)
	if err != nil {
		logger.Error("Failed to initialize Phase3RoboTorqUnitAssembler", "error", err)
		return nil, fmt.Errorf("failed to create Phase3RoboTorqUnitAssembler: %w", err)
	}
	logger.Info("Phase3RoboTorqUnitAssembler initialized", "channel_capacity", 10)

	// Create Phase3DistoDamPublisher (Phase 2 Milestone 5a: DistoDam publisher)
	phase3PublisherMetrics := mint.NewPhase3DistoDamPublisherMetrics(prometheus.DefaultRegisterer)
	phase3Publisher := mint.NewPhase3DistoDamPublisher(
		client.GetConnection(),
		phase3Assembler.GetUnitChannel(),
		logger,
		phase3PublisherMetrics,
	)
	logger.Info("Phase3DistoDamPublisher initialized", "topic", "distodam.units")

	// Create VerificationHandler (Phase 5: Merkle proof verification API + SPHINCS+ signature verification)
	verificationMetrics := mint.NewVerificationMetrics(nil) // TODO: Register with Prometheus registry
	verificationHandler := mint.NewVerificationHandler(
		phase3Assembler.GetProofCache(),
		phase3Assembler.GetSignatureArchive(),
		phase3Assembler.GetPublicKey(),
		":8080", // Verification + Main HTTP API unified port
		logger,
		verificationMetrics,
	)
	logger.Info("VerificationHandler initialized", "port", ":8080")

	return &Components{
		Client:              client,
		Phase2Receiver:      phase2Receiver,
		IngotHashQueue:      ingotHashQueue,
		Level2MerkleBuilder: level2MerkleBuilder,
		Phase3Assembler:     phase3Assembler,
		Phase3Publisher:     phase3Publisher,
		VerificationHandler: verificationHandler,
	}, nil
}

// startComponents starts all service components in the correct order (Phase 2/3 only)
func startComponents(ctx context.Context, components *Components, errChan chan error) error {
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

	// Start Phase2IngotReceiver (NATS subscriber for hash-only ingots from Refinery)
	if err := components.Phase2Receiver.Start(); err != nil {
		return fmt.Errorf("failed to start Phase2IngotReceiver: %w", err)
	}

	// Start Phase3RoboTorqUnitAssembler (Phase 2 Milestone 4b: RT unit assembler)
	go func() {
		components.Phase3Assembler.Start(ctx)
	}()

	// Start Phase3DistoDamPublisher (Phase 2 Milestone 5a: DistoDam publisher)
	go func() {
		components.Phase3Publisher.Start(ctx)
	}()

	// Start VerificationHandler (Phase 5: Merkle proof verification API)
	go func() {
		if err := components.VerificationHandler.Start(); err != nil && err != http.ErrServerClosed {
			errChan <- fmt.Errorf("VerificationHandler error: %w", err)
		}
	}()

	// Give components time to start
	time.Sleep(100 * time.Millisecond)

	return nil
}

// gracefulShutdown performs coordinated shutdown of all components (Phase 2/3 only)
func gracefulShutdown(ctx context.Context, components *Components, logger *slog.Logger) error {
	var shutdownErr error

	// Step 1: Stop Phase2IngotReceiver (NATS subscriber)
	logger.Info("Stopping Phase2IngotReceiver...")
	if err := components.Phase2Receiver.Stop(); err != nil {
		logger.Error("Error stopping Phase2IngotReceiver", "error", err)
		shutdownErr = err
	} else {
		logger.Info("Phase2IngotReceiver stopped")
	}

	// Step 1b: Stop VerificationHandler (HTTP server)
	logger.Info("Stopping VerificationHandler...")
	if err := components.VerificationHandler.Shutdown(ctx); err != nil {
		logger.Error("Error shutting down VerificationHandler", "error", err)
		shutdownErr = err
	} else {
		logger.Info("VerificationHandler stopped")
	}

	// Step 2: Drain Phase3RoboTorqUnitAssembler via context cancellation
	// Phase3Assembler.Start() and Phase3Publisher.Start() both respect context.Done()
	logger.Info("Draining Phase3RoboTorqUnitAssembler and Phase3DistoDamPublisher...")
	logger.Info("(Graceful shutdown via context cancellation)")

	// Step 3: Close NATS connection (drain pending publishes)
	logger.Info("Closing NATS connection...")
	if err := components.Client.Close(); err != nil {
		logger.Error("Error closing NATS connection", "error", err)
		shutdownErr = err
	} else {
		logger.Info("NATS connection closed")
	}

	return shutdownErr
}

// shutdownComponents performs emergency shutdown (no graceful handling) - Phase 2/3 only
func shutdownComponents(components *Components, logger *slog.Logger) {
	logger.Warn("Performing emergency shutdown...")

	if components.Phase2Receiver != nil {
		components.Phase2Receiver.Stop()
	}

	if components.VerificationHandler != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		components.VerificationHandler.Shutdown(ctx)
		cancel()
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
