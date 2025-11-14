package opportunity

import "sync"

type Opportunity struct {
	mu          sync.RWMutex
	ID          string `json:"id"`
	Builder     string `json:"builder"`
	Description string `json:"description"`

	// Mutable fields that might be updated during analysis
	RequiredRT int            `json:"required_rt"`
	Score      float64        `json:"score,omitempty"`
	Status     string         `json:"status,omitempty"`
	Metadata   map[string]any `json:"metadata,omitempty"`
}

// New creates a new Opportunity instance
func New(id, builder string, description string, requiredRT int, status string) *Opportunity {
	return &Opportunity{
		ID:          id,
		Builder:     builder,
		Description: description,
		RequiredRT:  requiredRT,
		Status:      status,
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
