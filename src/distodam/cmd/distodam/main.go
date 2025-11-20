// cmd/distodam/main.go - DistoDam service orchestration
// Phase 3: Complete service with dual-vault architecture
package main

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"b2b/distorouter/internal/config"
	"b2b/distorouter/internal/distodam"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

func main() {
	// ════════════════════════════════════════════════════════════
	// INITIALIZATION
	// ════════════════════════════════════════════════════════════

	// Structured JSON logging
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	logger.Info("starting distodam service",
		"version", "phase3-dual-vault",
		"architecture", "UBD")

	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		logger.Error("configuration failed", "error", err)
		os.Exit(1)
	}
	logger.Info("configuration loaded",
		"nats_url", cfg.NatsURL,
		"http_port", cfg.HTTPPort,
		"loan_policy", cfg.LoanPolicy)

	// Create context for graceful shutdown
	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	// ════════════════════════════════════════════════════════════
	// NATS CONNECTION
	// ════════════════════════════════════════════════════════════

	nc, err := nats.Connect(cfg.NatsURL,
		nats.MaxReconnects(-1),
		nats.ReconnectWait(2*time.Second),
		nats.DisconnectErrHandler(func(nc *nats.Conn, err error) {
			logger.Warn("nats disconnected", "error", err)
		}),
		nats.ReconnectHandler(func(nc *nats.Conn) {
			logger.Info("nats reconnected", "url", nc.ConnectedUrl())
		}),
	)
	if err != nil {
		logger.Error("nats connection failed", "error", err)
		os.Exit(1)
	}
	defer nc.Close()

	logger.Info("nats connected",
		"url", cfg.NatsURL,
		"server_name", nc.ConnectedServerName())

	// ════════════════════════════════════════════════════════════
	// METRICS & OBSERVABILITY
	// ════════════════════════════════════════════════════════════

	vaultMetrics := distodam.NewVaultMetrics()
	receiverMetrics := distodam.NewReceiverMetrics()

	logger.Info("metrics initialized",
		"vault_metrics", "enabled",
		"receiver_metrics", "enabled")

	// ════════════════════════════════════════════════════════════
	// DUAL-VAULT ARCHITECTURE
	// ════════════════════════════════════════════════════════════

	// Create certificate-based vault (stores actual Phase3RoboTorqUnit certificates)
	certificateVault := distodam.NewCertificateVaultClient(
		cfg.InitialStakeVaultRT,
		logger,
		vaultMetrics,
	)

	// Create legacy vault client for compatibility (used by VaultManager for loans)
	vaultClient := distodam.NewMockVaultClient(
		cfg.InitialStakeVaultRT,
		cfg.InitialDistoVaultRT,
		logger,
		vaultMetrics,
	)

	// Create event publisher for NATS
	eventPublisher := distodam.NewEventPublisher(nc, logger, vaultMetrics)

	// Create vault manager (orchestrates StakeVault + DistoVault + loans)
	vaultManager := distodam.NewVaultManager(
		vaultClient,
		cfg,
		logger,
		vaultMetrics,
	)

	logger.Info("dual-vault system initialized",
		"stake_vault_rt", 0.0,
		"disto_vault_certificates", 0,
		"outstanding_loans", 0)

	// ════════════════════════════════════════════════════════════
	// INPUT RECEIVERS (NATS Subscribers)
	// ════════════════════════════════════════════════════════════

	// MintEventReceiver: Receives RoboStake from Mint, deposits to StakeVault
	// NOW ALSO: Receives Phase3RoboTorqUnit certificates, deposits to DistoVault!
	mintReceiver := distodam.NewMintEventReceiver(
		nc,
		logger,
		vaultManager,
		certificateVault, // ← Certificate-based storage
		receiverMetrics,
		cfg.MintBatchesTopic, // "mint.batches"
	)

	// ContractFunder: Receives approved contracts, funds from vaults
	contractFunder := distodam.NewContractFunder(
		nc,
		logger,
		vaultManager,
		eventPublisher,
		receiverMetrics,
		vaultMetrics,
		cfg.ContractsApprovedTopic, // "contracts.approved"
		cfg.DamID,                  // "distodam-001"
	)

	// UBDRegistryReceiver: Receives UBD wallet registrations (Phase 4+)
	// TODO: Implement when UBD distribution is ready
	// ubdReceiver := distodam.NewUBDRegistryReceiver(...)

	// WalletRegistry: Tracks active wallets for distribution
	walletRegistry := distodam.NewWalletRegistry(
		nc,
		logger,
		cfg.WalletActivationTopic, // "wallet.activate"
	)

	// WalletDistributor: Pulls from DistoVault and distributes to active wallets
	// NOW USES: CertificateVault to withdraw actual Phase3RoboTorqUnit certificates!
	walletDistributor := distodam.NewWalletDistributor(
		nc,
		logger,
		cfg,
		vaultManager,
		certificateVault, // ← Certificate-based distribution
		walletRegistry,
		vaultMetrics,
	)

	logger.Info("input receivers created",
		"mint_receiver_topic", cfg.MintBatchesTopic,
		"contract_funder_topic", cfg.ContractsApprovedTopic,
		"wallet_activation_topic", cfg.WalletActivationTopic,
		"wallet_distribution_enabled", cfg.WalletDistributionEnabled)

	// ════════════════════════════════════════════════════════════
	// HTTP SERVER (Health, Status, Metrics)
	// ════════════════════════════════════════════════════════════

	mux := http.NewServeMux()

	// Health check endpoint
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("OK"))
	})

	// Status endpoint (human-readable vault state)
	mux.HandleFunc("/status", func(w http.ResponseWriter, r *http.Request) {
		stakeBalance, _ := vaultClient.GetStakeVaultBalance()
		distoBalance, _ := vaultClient.GetDistoVaultBalance()

		status := map[string]interface{}{
			"service": "distodam",
			"version": "phase3-dual-vault",
			"vaults": map[string]interface{}{
				"stake_vault_rt": stakeBalance,
				"disto_vault_rt": distoBalance,
			},
			"loans": map[string]interface{}{
				"outstanding_count": len(vaultManager.GetOutstandingLoans()),
			},
			"uptime_seconds": time.Since(time.Now()).Seconds(),
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(status)
	})

	// Prometheus metrics endpoint
	mux.Handle("/metrics", promhttp.Handler())

	// Start HTTP server
	httpServer := &http.Server{
		Addr:    ":" + cfg.HTTPPort,
		Handler: mux,
	}

	go func() {
		logger.Info("http server starting",
			"port", cfg.HTTPPort,
			"endpoints", []string{"/health", "/status", "/metrics"})

		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Error("http server failed", "error", err)
		}
	}()

	// ════════════════════════════════════════════════════════════
	// START RECEIVERS (NATS event loops)
	// ════════════════════════════════════════════════════════════

	// Start MintEventReceiver in background
	go func() {
		logger.Info("starting mint event receiver")
		if err := mintReceiver.Start(ctx); err != nil {
			logger.Error("mint receiver failed", "error", err)
		}
	}()

	// Start ContractFunder in background
	go func() {
		logger.Info("starting contract funder")
		if err := contractFunder.Start(ctx); err != nil {
			logger.Error("contract funder failed", "error", err)
		}
	}()

	// Start WalletRegistry in background (tracks activation messages)
	go func() {
		logger.Info("starting wallet registry")
		if err := walletRegistry.Start(ctx); err != nil {
			logger.Error("wallet registry failed", "error", err)
		}
	}()

	// Start WalletDistributor in background (minute-by-minute distributions)
	go func() {
		logger.Info("starting wallet distributor")
		if err := walletDistributor.Start(ctx); err != nil {
			logger.Error("wallet distributor failed", "error", err)
		}
	}()

	// Give receivers time to subscribe
	time.Sleep(500 * time.Millisecond)

	logger.Info("distodam service running",
		"receivers", []string{"mint_events", "contracts", "wallet_activations"},
		"distributors", []string{"wallet_distributor"},
		"http_port", cfg.HTTPPort)

	// ════════════════════════════════════════════════════════════
	// GRACEFUL SHUTDOWN
	// ════════════════════════════════════════════════════════════

	<-ctx.Done()
	logger.Info("shutdown signal received, initiating graceful shutdown")

	// Create shutdown timeout context
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer shutdownCancel()

	// Shutdown HTTP server
	if err := httpServer.Shutdown(shutdownCtx); err != nil {
		logger.Error("http server shutdown error", "error", err)
	}

	// Shutdown receivers (they'll drain pending messages)
	logger.Info("shutting down receivers")
	if err := mintReceiver.Shutdown(); err != nil {
		logger.Error("mint receiver shutdown error", "error", err)
	}
	if err := contractFunder.Shutdown(); err != nil {
		logger.Error("contract funder shutdown error", "error", err)
	}
	if err := walletRegistry.Shutdown(); err != nil {
		logger.Error("wallet registry shutdown error", "error", err)
	}
	if err := walletDistributor.Shutdown(); err != nil {
		logger.Error("wallet distributor shutdown error", "error", err)
	}

	// Flush NATS messages
	if err := nc.Flush(); err != nil {
		logger.Error("nats flush error", "error", err)
	}

	logger.Info("distodam service stopped gracefully",
		"final_stake_vault_rt", func() float64 { b, _ := vaultClient.GetStakeVaultBalance(); return b }(),
		"final_disto_vault_rt", func() float64 { b, _ := vaultClient.GetDistoVaultBalance(); return b }())
}
