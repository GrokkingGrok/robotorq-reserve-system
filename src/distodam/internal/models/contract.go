package models

import "time"

// Contract represents a contract that DistoDam will fund
// This mirrors the contract structure from the Trust service
type Contract struct {
	// Immutable fields
	ID            string  `json:"id"`
	OpportunityID string  `json:"opportunity_id"`
	Builder       string  `json:"builder"`
	DiggerURL     string  `json:"digger_url"`
	RoboStake     float64 `json:"robo_stake"`
	ROI           float64 `json:"roi"`
	Torq          int     `json:"torq"`

	// Mutable fields
	Status string `json:"status"` // "bidding", "financing", "funded", "executing", "produced", "completed", "failed"

	// Timestamps
	CreatedAt   time.Time  `json:"created_at"`
	FundedAt    *time.Time `json:"funded_at,omitempty"`
	ExecutedAt  *time.Time `json:"executed_at,omitempty"`
	CompletedAt *time.Time `json:"completed_at,omitempty"`

	// Legacy fields (for compatibility)
	MaxTokenThroughput int `json:"max_token_throughput,omitempty"`
	IntervalSeconds    int `json:"interval_seconds,omitempty"`
	TotalTokens        int `json:"total_tokens,omitempty"`
}
