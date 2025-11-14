// internal/refinery/ore_receiver.go
// Handles incoming JouleTorqOre from Digger

package refinery

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"

	"b2b/refinery/internal/models"
)

// QueueAdder defines the interface for adding items to queues
type QueueAdder interface {
	AddJoule(item *models.JouleQueueItem) error
	AddRobo(item *models.RoboQueueItem) error
}

// OreReceiver handles incoming JouleTorqOre from Digger
type OreReceiver struct {
	queueMgr QueueAdder
}

// NewOreReceiver creates a new ore receiver
func NewOreReceiver(queueMgr QueueAdder) *OreReceiver {
	return &OreReceiver{
		queueMgr: queueMgr,
	}
}

// ReceiveOre processes incoming ore and adds to queues
func (o *OreReceiver) ReceiveOre(ore *models.JouleTorqOre) error {
	// Validate the ore structure
	if err := ore.Validate(); err != nil {
		return fmt.Errorf("ore validation failed: %w", err)
	}

	slog.Info("ore received from digger",
		"contract", ore.ContractID,
		"digger", ore.DiggerID,
		"milestone", ore.MilestoneIndex,
		"joules", ore.Joules,
		"robo_stake", ore.RoboStakeAmount,
		"tokens", ore.TokensGenerated,
	)

	// Compute hash of the joule contribution (for merkle tree)
	jouleHash := hashOre(ore)

	// Create JouleQueueItem and add to queue
	jouleItem := models.NewJouleQueueItem(
		float64(ore.Joules),
		ore.ContractID,
		jouleHash,
	)
	if err := o.queueMgr.AddJoule(&jouleItem); err != nil {
		slog.Warn("joule queue full",
			"contract", ore.ContractID,
			"amount", jouleItem.Amount,
			"error", err,
		)
		return fmt.Errorf("joule queue full: %w", err)
	}

	// Calculate price (tokens generated per RoboTorq staked)
	price := ore.CalculatePrice()

	// Create RoboQueueItem and add to queue
	roboItem := models.NewRoboQueueItem(
		ore.RoboStakeAmount,
		price,
		ore.ContractID,
	)
	if err := o.queueMgr.AddRobo(&roboItem); err != nil {
		slog.Warn("robo queue full",
			"contract", ore.ContractID,
			"amount", roboItem.Amount,
			"price", roboItem.Price,
			"error", err,
		)
		return fmt.Errorf("robo queue full: %w", err)
	}

	return nil
}

// hashOre computes a SHA256 hash of the ore data for merkle tree
func hashOre(ore *models.JouleTorqOre) string {
	// Create a consistent representation for hashing
	data := fmt.Sprintf("%s:%s:%d:%d:%d",
		ore.ContractID,
		ore.DiggerID,
		ore.MilestoneIndex,
		ore.Joules,
		ore.Timestamp,
	)
	hash := sha256.Sum256([]byte(data))
	return hex.EncodeToString(hash[:])
}

// HTTPHandler creates an HTTP handler for the /receive-ore endpoint
func (o *OreReceiver) HTTPHandler(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var ore models.JouleTorqOre
	if err := json.NewDecoder(req.Body).Decode(&ore); err != nil {
		http.Error(w, fmt.Sprintf("Invalid JSON: %v", err), http.StatusBadRequest)
		return
	}

	// Process the ore
	if err := o.ReceiveOre(&ore); err != nil {
		// Check if it's a queue-full error using errors.Is
		if errors.Is(err, models.ErrQueueFull) {
			http.Error(w, err.Error()+", retry later", http.StatusTooManyRequests)
			return
		}
		// Validation or other errors
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Success response
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]interface{}{
		"status":      "accepted",
		"contract_id": ore.ContractID,
		"milestone":   ore.MilestoneIndex,
	})
}
