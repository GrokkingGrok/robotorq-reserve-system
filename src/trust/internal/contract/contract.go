package contract

import (
	"sync"
	"time"
)

// Contract represents an approved opportunity that has been converted into a binding agreement.
// Most fields are immutable after creation - only Status and timestamp fields change.
type Contract struct {
	mu sync.RWMutex

	// Immutable fields (set at creation)
	ID            string  `json:"id"`
	OpportunityID string  `json:"opportunity_id"`
	Builder       string  `json:"builder"`
	DiggerURL     string  `json:"digger_url"`
	RoboStake     float64 `json:"robo_stake"`
	ROI           float64 `json:"roi"`
	Torq          int     `json:"torq"`

	// Mutable fields (updated as contract progresses)
	Status string `json:"status"` // "bidding", "financing", "funded", "executing", "produced", "completed", "failed"

	// Timestamps (set once when status changes)
	CreatedAt   time.Time  `json:"created_at"`
	FundedAt    *time.Time `json:"funded_at,omitempty"`
	ExecutedAt  *time.Time `json:"executed_at,omitempty"`
	CompletedAt *time.Time `json:"completed_at,omitempty"`

	// Legacy fields from original Contract (kept for compatibility)
	MaxTokenThroughput int `json:"max_token_throughput,omitempty"`
	IntervalSeconds    int `json:"interval_seconds,omitempty"`
	TotalTokens        int `json:"total_tokens,omitempty"`
}

// New creates a new Contract from an approved Opportunity
func New(id, opportunityID, builder, diggerURL string, roboStake, roi float64, torq int) *Contract {
	return &Contract{
		ID:            id,
		OpportunityID: opportunityID,
		Builder:       builder,
		DiggerURL:     diggerURL,
		RoboStake:     roboStake,
		ROI:           roi,
		Torq:          torq,
		Status:        "bidding", // New contracts start in bidding phase
		CreatedAt:     time.Now(),
	}
}

// UpdateStatus safely updates the contract status and sets appropriate timestamp
func (c *Contract) UpdateStatus(status string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	
	c.Status = status
	now := time.Now()

	// Set timestamp based on status
	switch status {
	case "funded":
		if c.FundedAt == nil {
			c.FundedAt = &now
		}
	case "executing":
		if c.ExecutedAt == nil {
			c.ExecutedAt = &now
		}
	case "completed", "produced":
		if c.CompletedAt == nil {
			c.CompletedAt = &now
		}
	}
}

// GetStatus safely reads the contract status
func (c *Contract) GetStatus() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.Status
}

// GetDiggerURL safely reads the Digger URL
func (c *Contract) GetDiggerURL() string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.DiggerURL
}

// GetRoboStake safely reads the RoboStake amount
func (c *Contract) GetRoboStake() float64 {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.RoboStake
}
