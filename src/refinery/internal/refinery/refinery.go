// internal/refinery/refinery.go
// Core Refinery service orchestration

package refinery

import (
	"log/slog"
)

// Refinery is the main service struct
type Refinery struct {
	queueManager   *QueueManager
	ingotAssembler *Phase2IngotAssembler
	mintClient     *MintClient
}

// NewRefinery creates a new Refinery instance
func NewRefinery(qm *QueueManager, ia *Phase2IngotAssembler, mc *MintClient) *Refinery {
	return &Refinery{
		queueManager:   qm,
		ingotAssembler: ia,
		mintClient:     mc,
	}
}

// Start begins all background workers
func (r *Refinery) Start() {
	slog.Info("starting refinery service")
	// TODO: Start ingot assembler
	// TODO: Start any other background workers
}

// Stop gracefully shuts down the service
func (r *Refinery) Stop() {
	slog.Info("stopping refinery service")
	// TODO: Stop workers gracefully
	// TODO: Close NATS connection
}
