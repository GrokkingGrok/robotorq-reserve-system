// internal/models/queue.go
// Data structures for internal queue management

package models

import "time"

// JouleQueueItem represents a single joule contribution waiting to be assembled into an ingot.
// These are queued as ore arrives and consumed by the IngotAssembler.
type JouleQueueItem struct {
	// Amount is the number of joules from this ore
	// Example: 1250.0 (from a 5-second milestone)
	Amount float64

	// ContractID identifies which contract this joule contribution came from
	// Example: "test-contract-001"
	// Used for tracking which contracts contribute to each ingot
	ContractID string

	// Timestamp is when this item was queued
	// Used for monitoring queue latency and FIFO ordering
	Timestamp time.Time

	// Hash is the SHA256 hash of the source ore
	// Used for merkle tree construction
	// Format: hex-encoded string (64 characters)
	Hash string
}

// RoboQueueItem represents a RoboStake contribution waiting to be paired with joules.
// These are queued separately and matched with joules during ingot assembly.
type RoboQueueItem struct {
	// Amount is the RoboStake amount in RT (fractional)
	// Example: 0.00416 RT
	Amount float64

	// Price is the value ratio (tokens per RT)
	// Calculated as: tokens_generated / robo_stake_amount
	// Example: 14,400 (60 tokens / 0.00416 RT)
	Price float64

	// ContractID identifies which contract this RoboStake came from
	// Example: "test-contract-001"
	ContractID string

	// Timestamp is when this item was queued
	Timestamp time.Time
}

// NewJouleQueueItem creates a new joule queue item
func NewJouleQueueItem(amount float64, contractID string, hash string) JouleQueueItem {
	return JouleQueueItem{
		Amount:     amount,
		ContractID: contractID,
		Hash:       hash,
		Timestamp:  time.Now().UTC(),
	}
}

// NewRoboQueueItem creates a new robo queue item
func NewRoboQueueItem(amount float64, price float64, contractID string) RoboQueueItem {
	return RoboQueueItem{
		Amount:     amount,
		Price:      price,
		ContractID: contractID,
		Timestamp:  time.Now().UTC(),
	}
}
