package opportunity

import "sync"

type Opportunity struct {
	mu          sync.RWMutex
	ID          string `json:"id"`
	Builder     string `json:"builder"`
	Description string `json:"description"`
	DiggerURL   string `json:"digger_url"` // URL of the builder's Digger service

	// Mutable fields that might be updated during analysis
	RequiredRT int            `json:"required_rt"`
	ROI        float64        `json:"roi"`           // Expected return on investment
	Score      float64        `json:"score,omitempty"` // Internal appraisal score
	Status     string         `json:"status,omitempty"` // "lead", "appraising", "approved", "rejected"
	Metadata   map[string]any `json:"metadata,omitempty"` // Optional/experimental fields
}

// New creates a new Opportunity instance
func New(id, builder, description, diggerURL string, requiredRT int, roi float64) *Opportunity {
	return &Opportunity{
		ID:          id,
		Builder:     builder,
		Description: description,
		DiggerURL:   diggerURL,
		RequiredRT:  requiredRT,
		ROI:         roi,
		Status:      "lead", // All opportunities start as leads
		Metadata:    make(map[string]any),
	}
}

// UpdateScore safely updates the opportunity score
func (o *Opportunity) UpdateScore(score float64) {
	o.mu.Lock()
	defer o.mu.Unlock()
	o.Score = score
}

// GetScore safely reads the opportunity score
func (o *Opportunity) GetScore() float64 {
	o.mu.RLock()
	defer o.mu.RUnlock()
	return o.Score
}

// UpdateStatus safely updates the opportunity status
func (o *Opportunity) UpdateStatus(status string) {
	o.mu.Lock()
	defer o.mu.Unlock()
	o.Status = status
}

// GetStatus safely reads the opportunity status
func (o *Opportunity) GetStatus() string {
	o.mu.RLock()
	defer o.mu.RUnlock()
	return o.Status
}

// SetMetadata safely sets a metadata value
func (o *Opportunity) SetMetadata(key string, value any) {
	o.mu.Lock()
	defer o.mu.Unlock()
	o.Metadata[key] = value
}

// GetMetadata safely gets a metadata value
func (o *Opportunity) GetMetadata(key string) (any, bool) {
	o.mu.RLock()
	defer o.mu.RUnlock()
	val, ok := o.Metadata[key]
	return val, ok
}

// GetROI safely reads the ROI
func (o *Opportunity) GetROI() float64 {
	o.mu.RLock()
	defer o.mu.RUnlock()
	return o.ROI
}

// UpdateROI safely updates the ROI
func (o *Opportunity) UpdateROI(roi float64) {
	o.mu.Lock()
	defer o.mu.Unlock()
	o.ROI = roi
}

// GetDiggerURL safely reads the Digger URL
func (o *Opportunity) GetDiggerURL() string {
	o.mu.RLock()
	defer o.mu.RUnlock()
	return o.DiggerURL
}
