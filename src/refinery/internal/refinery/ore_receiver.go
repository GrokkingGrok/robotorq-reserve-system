// internal/refinery/ore_receiver.go
// Handles incoming JouleTorqOre from Digger

package refinery

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"

	"b2b/refinery/internal/models"
)

// QueueAdder defines the interface for adding units to queue
type QueueAdder interface {
	AddUnit(unit *models.JouleTorqUnit) error
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

// ReceiveOre processes incoming ore and creates JouleTorqUnits
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

	// Convert JouleTorqOre (batch of work) → multiple JouleTorqUnits (atomic units)
	// Each unit represents processing of 1 token
	// Distribute joules and robo stake evenly across all tokens generated
	if ore.TokensGenerated == 0 {
		return fmt.Errorf("no tokens generated in ore")
	}

	joulesPerToken := float64(ore.Joules) / float64(ore.TokensGenerated)
	roboPerToken := ore.RoboStakeAmount / float64(ore.TokensGenerated)

	// Convert signature bytes to hex string (empty if nil)
	signatureHex := ""
	if ore.Signature != nil && len(ore.Signature) > 0 {
		signatureHex = hex.EncodeToString(ore.Signature)
	}

	// Create one JouleTorqUnit for each token generated
	for tokenIndex := 0; tokenIndex < int(ore.TokensGenerated); tokenIndex++ {
		unit := models.NewJouleTorqUnit(
			ore.ContractID,
			int(ore.MilestoneIndex),
			tokenIndex,
			joulesPerToken,
			roboPerToken,
			ore.DiggerID,
			signatureHex, // Stub: Ore doesn't have per-token signatures yet
			"",           // Stub: Ore doesn't include digger public key yet
		)

		// Add unit to queue
		if err := o.queueMgr.AddUnit(unit); err != nil {
			slog.Warn("unit queue full",
				"contract", ore.ContractID,
				"token_id", unit.TokenID,
				"error", err,
			)
			return fmt.Errorf("unit queue full: %w", err)
		}
	}

	return nil
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
