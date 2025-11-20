// wallet_registry.go - Tracks active wallets for distribution
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/nats-io/nats.go"
)

// WalletRegistry tracks which wallets are active for distribution
type WalletRegistry struct {
	nc           *nats.Conn
	logger       *slog.Logger
	topic        string
	subscription *nats.Subscription

	// Active wallets (wallet_id → activation time)
	activeWallets map[string]time.Time
	mu            sync.RWMutex

	ctx    context.Context
	cancel context.CancelFunc
}

// NewWalletRegistry creates a new WalletRegistry
func NewWalletRegistry(
	nc *nats.Conn,
	logger *slog.Logger,
	topic string,
) *WalletRegistry {
	ctx, cancel := context.WithCancel(context.Background())

	return &WalletRegistry{
		nc:            nc,
		logger:        logger,
		topic:         topic,
		activeWallets: make(map[string]time.Time),
		ctx:           ctx,
		cancel:        cancel,
	}
}

// Start begins listening for wallet activation messages
func (wr *WalletRegistry) Start(ctx context.Context) error {
	wr.logger.Info("starting wallet registry",
		"topic", wr.topic)

	// Subscribe to activation topic
	sub, err := wr.nc.Subscribe(wr.topic, func(msg *nats.Msg) {
		wr.handleActivation(msg)
	})

	if err != nil {
		wr.logger.Error("failed to subscribe to wallet activations",
			"topic", wr.topic,
			"error", err)
		return fmt.Errorf("subscribe to %s: %w", wr.topic, err)
	}

	wr.subscription = sub

	wr.logger.Info("wallet registry started",
		"topic", wr.topic)

	// Block until context cancelled
	<-ctx.Done()

	wr.logger.Info("wallet registry stopping due to context cancellation")
	return wr.Shutdown()
}

// handleActivation processes a wallet activation message
func (wr *WalletRegistry) handleActivation(msg *nats.Msg) {
	wr.logger.Debug("received wallet activation message",
		"subject", msg.Subject,
		"size_bytes", len(msg.Data))

	// Parse activation message
	var activation WalletActivationMessage
	if err := json.Unmarshal(msg.Data, &activation); err != nil {
		wr.logger.Error("failed to parse wallet activation",
			"error", err,
			"data_preview", string(msg.Data[:min(100, len(msg.Data))]))
		msg.Nak()
		return
	}

	wr.mu.Lock()
	defer wr.mu.Unlock()

	if activation.Activate {
		// Activate wallet
		wr.activeWallets[activation.WalletID] = activation.RequestedAt
		wr.logger.Info("wallet activated",
			"wallet_id", activation.WalletID,
			"total_active", len(wr.activeWallets))
	} else {
		// Deactivate wallet
		delete(wr.activeWallets, activation.WalletID)
		wr.logger.Info("wallet deactivated",
			"wallet_id", activation.WalletID,
			"total_active", len(wr.activeWallets))
	}

	msg.Ack()
}

// GetActiveWallets returns a copy of all active wallet IDs
func (wr *WalletRegistry) GetActiveWallets() []string {
	wr.mu.RLock()
	defer wr.mu.RUnlock()

	wallets := make([]string, 0, len(wr.activeWallets))
	for walletID := range wr.activeWallets {
		wallets = append(wallets, walletID)
	}

	return wallets
}

// IsActive checks if a wallet is currently active
func (wr *WalletRegistry) IsActive(walletID string) bool {
	wr.mu.RLock()
	defer wr.mu.RUnlock()

	_, exists := wr.activeWallets[walletID]
	return exists
}

// ActiveCount returns the number of active wallets
func (wr *WalletRegistry) ActiveCount() int {
	wr.mu.RLock()
	defer wr.mu.RUnlock()

	return len(wr.activeWallets)
}

// Shutdown stops the WalletRegistry
func (wr *WalletRegistry) Shutdown() error {
	wr.logger.Info("shutting down wallet registry")

	if wr.subscription != nil {
		if err := wr.subscription.Unsubscribe(); err != nil {
			wr.logger.Error("failed to unsubscribe from wallet activations",
				"error", err)
			return fmt.Errorf("unsubscribe: %w", err)
		}

		wr.subscription = nil
	}

	wr.cancel()

	wr.logger.Info("wallet registry shutdown complete")
	return nil
}
