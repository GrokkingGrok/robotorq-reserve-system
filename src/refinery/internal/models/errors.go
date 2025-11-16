// internal/models/errors.go
// Common error types for Refinery models

package models

import "errors"

// Validation errors for JouleTorqOre
var (
	ErrInvalidDiggerID   = errors.New("digger_id cannot be empty")
	ErrInvalidContractID = errors.New("contract_id cannot be empty")
	ErrZeroJoules        = errors.New("joules must be greater than zero")
	ErrNegativeRoboStake = errors.New("robo_stake_amount cannot be negative")
	ErrZeroTimestamp     = errors.New("timestamp cannot be zero")
)

// Validation errors for TokenTorqIngot
var (
	ErrInvalidIngotID    = errors.New("ingot_id cannot be empty")
	ErrInvalidJouleTotal = errors.New("joule_torq_total must be exactly 3600")
	ErrNoContracts       = errors.New("ingot must have at least one contributing contract")
	ErrNoHashes          = errors.New("ingot must have at least one joule hash")
)

// Queue errors
var (
	ErrQueueFull         = errors.New("queue is full, cannot accept more items")
	ErrQueueEmpty        = errors.New("queue is empty, no items to consume")
	ErrQueueShuttingDown = errors.New("queue is shutting down")
	ErrDeprecated        = errors.New("deprecated method called - use Phase 2 hash-based API")
)
