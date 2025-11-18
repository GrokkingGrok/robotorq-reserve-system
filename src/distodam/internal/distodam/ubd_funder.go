// ubd_registry_receiver.go - Receives UBD wallet registrations (stub for future implementation)
package distodam

import (
	"context"
	"log/slog"
)

// UBDRegistryReceiver handles UBD wallet registration events
// PHASE 6 MVP: Stub interface only - implementation pending Wallet service
// PHASE 7+: Full implementation with NATS subscription to ubd.wallet.registered
//
// Purpose: Track which wallets are eligible for UBD distributions
// When DistoVault has surplus, DistoDam distributes equally to registered wallets
type UBDRegistryReceiver struct {
	logger *slog.Logger
}

// NewUBDRegistryReceiver creates a new UBDRegistryReceiver stub
func NewUBDRegistryReceiver(logger *slog.Logger) *UBDRegistryReceiver {
	return &UBDRegistryReceiver{
		logger: logger,
	}
}

// Start begins listening for wallet registrations (stub - does nothing in Phase 6)
func (urr *UBDRegistryReceiver) Start(ctx context.Context) error {
	urr.logger.Info("UBDRegistryReceiver stub initialized (no-op in Phase 6 MVP)",
		"status", "waiting for Wallet service",
		"future_topic", "ubd.wallet.registered",
		"note", "Will track registered wallets for periodic distributions")

	// Block until context cancelled (no actual work)
	<-ctx.Done()
	
	return urr.Shutdown()
}

// Shutdown stops the UBDRegistryReceiver (stub)
func (urr *UBDRegistryReceiver) Shutdown() error {
	urr.logger.Info("UBDRegistryReceiver stub shutdown")
	return nil
}

// Future implementation outline (Phase 7+):
//
// type UBDWalletRegistration struct {
//     WalletID    string    `json:"wallet_id"`
//     MemberID    string    `json:"member_id"`
//     RegisteredAt time.Time `json:"registered_at"`
//     Active      bool      `json:"active"`
// }
//
// func (urr *UBDRegistryReceiver) handleWalletRegistration(msg *nats.Msg) {
//     // 1. Parse UBDWalletRegistration from JSON
//     // 2. Validate wallet ID format
//     // 3. Add to internal registry map[walletID]*UBDWalletRegistration
//     // 4. Update metrics (ubd_wallets_registered_total, ubd_wallets_active)
//     // 5. Log registration event
// }
//
// Periodic Distribution (separate component):
// - Every N hours (configurable), check DistoVault balance
// - If balance > threshold, calculate per-wallet distribution
// - Publish UBDDistributionEvent for each active wallet
// - Withdraw from DistoVault and send to wallet service
