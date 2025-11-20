// wallet_distributor.go - Distributes RT from DistoVault to active wallets
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	"b2b/distorouter/internal/config"

	"github.com/nats-io/nats.go"
)

// WalletDistributor pulls RT certificates from DistoVault and distributes to active wallets
type WalletDistributor struct {
	nc               *nats.Conn
	logger           *slog.Logger
	config           *config.Config
	vaultManager     *VaultManager
	certificateVault *CertificateVaultClient  // Certificate-based vault
	walletRegistry   *WalletRegistry
	metrics          *VaultMetrics

	distributionTopic string

	ctx    context.Context
	cancel context.CancelFunc
}

// NewWalletDistributor creates a new WalletDistributor
func NewWalletDistributor(
	nc *nats.Conn,
	logger *slog.Logger,
	cfg *config.Config,
	vaultManager *VaultManager,
	certificateVault *CertificateVaultClient,
	walletRegistry *WalletRegistry,
	metrics *VaultMetrics,
) *WalletDistributor {
	ctx, cancel := context.WithCancel(context.Background())

	return &WalletDistributor{
		nc:               nc,
		logger:           logger,
		config:           cfg,
		vaultManager:     vaultManager,
		certificateVault: certificateVault,
		walletRegistry:   walletRegistry,
		metrics:          metrics,
		distributionTopic: cfg.WalletDistributionTopic,
		ctx:              ctx,
		cancel:           cancel,
	}
}

// Start begins the distribution loop (runs every minute)
func (wd *WalletDistributor) Start(ctx context.Context) error {
	if !wd.config.WalletDistributionEnabled {
		wd.logger.Info("wallet distribution disabled, not starting distributor")
		return nil
	}

	wd.logger.Info("starting wallet distributor",
		"distribution_topic", wd.distributionTopic,
		"rt_per_minute", wd.config.WalletDistributionRTPerMin)

	// Create ticker for 1-minute intervals
	ticker := time.NewTicker(1 * time.Minute)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			wd.logger.Info("wallet distributor stopping due to context cancellation")
			return wd.Shutdown()

		case <-ticker.C:
			// Run distribution cycle
			if err := wd.distributeCycle(); err != nil {
				wd.logger.Error("distribution cycle failed",
					"error", err)
				// Don't stop on error, continue with next cycle
			}
		}
	}
}

// distributeCycle pulls RT certificates from DistoVault and distributes to active wallets
func (wd *WalletDistributor) distributeCycle() error {
	activeWallets := wd.walletRegistry.GetActiveWallets()

	if len(activeWallets) == 0 {
		wd.logger.Debug("no active wallets, skipping distribution")
		return nil
	}

	wd.logger.Info("starting distribution cycle",
		"active_wallets", len(activeWallets))

	// Check DistoVault certificate balance
	distoBalance, err := wd.certificateVault.GetDistoVaultBalance()
	if err != nil {
		return fmt.Errorf("get DistoVault balance: %w", err)
	}

	certsAvailable := int(distoBalance) // 1 certificate = 1 RT

	if certsAvailable == 0 {
		wd.logger.Warn("no RT certificates available in DistoVault, skipping distribution")
		return nil
	}

	// Distribute certificates to wallets (one certificate per wallet per minute)
	successCount := 0
	certsDistributed := 0

	for _, walletID := range activeWallets {
		if certsDistributed >= certsAvailable {
			wd.logger.Warn("ran out of certificates during distribution",
				"distributed", certsDistributed,
				"remaining_wallets", len(activeWallets)-successCount)
			break
		}

		// Withdraw an actual Phase3RoboTorqUnit certificate from DistoVault
		cert, err := wd.certificateVault.WithdrawFromDistoVaultCertificate()

		if err != nil {
			wd.logger.Error("failed to withdraw certificate from DistoVault",
				"wallet_id", walletID,
				"error", err)
			continue
		}

		if cert == nil {
			wd.logger.Warn("DistoVault returned nil certificate (empty)",
				"wallet_id", walletID)
			break
		}

		// Get updated balance
		remainingCerts, _ := wd.certificateVault.GetDistoVaultBalance()

		// Publish the actual certificate to wallet
		distMsg := &WalletDistributionMessage{
			WalletID:     walletID,
			RTUnit:       *cert, // Send the complete certificate!
			Timestamp:    time.Now(),
			DistoBalance: remainingCerts,
		}

		msgData, err := json.Marshal(distMsg)
		if err != nil {
			wd.logger.Error("failed to serialize distribution message",
				"wallet_id", walletID,
				"cert_unit_id", cert.UnitID,
				"error", err)
			// Certificate already withdrawn - this is bad!
			continue
		}

		// Publish to wallet
		if err := wd.nc.Publish(wd.distributionTopic, msgData); err != nil {
			wd.logger.Error("failed to publish distribution message",
				"wallet_id", walletID,
				"cert_unit_id", cert.UnitID,
				"error", err)
			// Certificate lost - TODO: Consider re-depositing or compensation
			continue
		}

		successCount++
		certsDistributed++

		wd.logger.Info("distributed RT certificate to wallet",
			"wallet_id", walletID,
			"cert_unit_id", cert.UnitID,
			"cert_merkle_root", cert.MerkleRoot,
			"disto_certificates_remaining", remainingCerts)
	}

	wd.logger.Info("distribution cycle complete",
		"successful_distributions", successCount,
		"certificates_distributed", certsDistributed,
		"active_wallets", len(activeWallets))

	return nil
}

// Shutdown stops the WalletDistributor
func (wd *WalletDistributor) Shutdown() error {
	wd.logger.Info("shutting down wallet distributor")
	wd.cancel()
	wd.logger.Info("wallet distributor shutdown complete")
	return nil
}
