// internal/refinery/health.go
// Health check endpoint for Refinery service

package refinery

import (
	"encoding/json"
	"net/http"
	"time"
)

// HealthChecker defines the interface for checking service health
type HealthChecker interface {
	GetQueueSize() int
	GetCapacity() int
}

// MintHealthChecker defines the interface for checking Mint client health
type MintHealthChecker interface {
	IsConnected() bool
	GetStatus() string
}

// AssemblerHealthChecker defines the interface for checking assembler health
type AssemblerHealthChecker interface {
	GetAccumulatedUnits() int
	GetCompletedIngotsCount() int
}

// HealthResponse represents the health check response
type HealthResponse struct {
	Status        string              `json:"status"`
	Timestamp     time.Time           `json:"timestamp"`
	Queue         QueueHealth         `json:"queue"`
	NATS          NATSHealth          `json:"nats"`
	IngotAssembly IngotAssemblyHealth `json:"ingot_assembly"`
}

// QueueHealth represents queue status
type QueueHealth struct {
	UnitQueueSize     int     `json:"unit_queue_size"`
	UnitQueueCapacity int     `json:"unit_queue_capacity"`
	UnitQueueUsage    float64 `json:"unit_queue_usage_percent"`
}

// NATSHealth represents NATS connection status
type NATSHealth struct {
	Connected bool   `json:"connected"`
	Status    string `json:"status"`
}

// IngotAssemblyHealth represents ingot assembly status
type IngotAssemblyHealth struct {
	AccumulatedUnits int     `json:"accumulated_units"`
	CompletedIngots  int     `json:"completed_ingots_pending"`
	ProgressPercent  float64 `json:"progress_to_next_ingot_percent"`
}

// HealthHandler provides health check functionality
type HealthHandler struct {
	queueMgr   HealthChecker
	mintClient MintHealthChecker
	assembler  AssemblerHealthChecker
}

// NewHealthHandler creates a new health handler
func NewHealthHandler(
	queueMgr HealthChecker,
	mintClient MintHealthChecker,
	assembler AssemblerHealthChecker,
) *HealthHandler {
	return &HealthHandler{
		queueMgr:   queueMgr,
		mintClient: mintClient,
		assembler:  assembler,
	}
}

// HTTPHandler handles HTTP health check requests
func (hh *HealthHandler) HTTPHandler(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Gather health metrics
	unitSize := hh.queueMgr.GetQueueSize()
	unitCapacity := hh.queueMgr.GetCapacity()

	// Calculate queue usage percentage
	unitUsage := 0.0
	if unitCapacity > 0 {
		unitUsage = (float64(unitSize) / float64(unitCapacity)) * 100
	}

	// Get assembler metrics
	accumulatedUnits := hh.assembler.GetAccumulatedUnits()
	completedIngots := hh.assembler.GetCompletedIngotsCount()

	// Calculate progress toward next ingot (3600 units threshold)
	progressPercent := 0.0
	if accumulatedUnits > 0 {
		progressPercent = (float64(accumulatedUnits) / 3600.0) * 100
		if progressPercent > 100 {
			progressPercent = 100 // Cap at 100%
		}
	}

	// Determine overall status
	status := "healthy"
	if !hh.mintClient.IsConnected() {
		status = "degraded" // NATS disconnected
	} else if unitUsage > 90 {
		status = "warning" // Queue near capacity
	}

	health := HealthResponse{
		Status:    status,
		Timestamp: time.Now().UTC(),
		Queue: QueueHealth{
			UnitQueueSize:     unitSize,
			UnitQueueCapacity: unitCapacity,
			UnitQueueUsage:    unitUsage,
		},
		NATS: NATSHealth{
			Connected: hh.mintClient.IsConnected(),
			Status:    hh.mintClient.GetStatus(),
		},
		IngotAssembly: IngotAssemblyHealth{
			AccumulatedUnits: accumulatedUnits,
			CompletedIngots:  completedIngots,
			ProgressPercent:  progressPercent,
		},
	}

	// Set HTTP status code based on health
	statusCode := http.StatusOK
	if status == "degraded" {
		statusCode = http.StatusServiceUnavailable
	} else if status == "warning" {
		statusCode = http.StatusOK // Still operational
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(health)
}
