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
	GetJouleQueueSize() int
	GetRoboQueueSize() int
	GetJouleCapacity() int
	GetRoboCapacity() int
}

// MintHealthChecker defines the interface for checking Mint client health
type MintHealthChecker interface {
	IsConnected() bool
	GetStatus() string
}

// AssemblerHealthChecker defines the interface for checking assembler health
type AssemblerHealthChecker interface {
	GetAccumulatedJoules() float64
	GetCompletedIngotsCount() int
}

// HealthResponse represents the health check response
type HealthResponse struct {
	Status        string              `json:"status"`
	Timestamp     time.Time           `json:"timestamp"`
	Queues        QueueHealth         `json:"queues"`
	NATS          NATSHealth          `json:"nats"`
	IngotAssembly IngotAssemblyHealth `json:"ingot_assembly"`
}

// QueueHealth represents queue status
type QueueHealth struct {
	JouleQueueSize     int     `json:"joule_queue_size"`
	JouleQueueCapacity int     `json:"joule_queue_capacity"`
	JouleQueueUsage    float64 `json:"joule_queue_usage_percent"`
	RoboQueueSize      int     `json:"robo_queue_size"`
	RoboQueueCapacity  int     `json:"robo_queue_capacity"`
	RoboQueueUsage     float64 `json:"robo_queue_usage_percent"`
}

// NATSHealth represents NATS connection status
type NATSHealth struct {
	Connected bool   `json:"connected"`
	Status    string `json:"status"`
}

// IngotAssemblyHealth represents ingot assembly status
type IngotAssemblyHealth struct {
	AccumulatedJoules float64 `json:"accumulated_joules"`
	CompletedIngots   int     `json:"completed_ingots_pending"`
	ProgressPercent   float64 `json:"progress_to_next_ingot_percent"`
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
	jouleSize := hh.queueMgr.GetJouleQueueSize()
	jouleCapacity := hh.queueMgr.GetJouleCapacity()
	roboSize := hh.queueMgr.GetRoboQueueSize()
	roboCapacity := hh.queueMgr.GetRoboCapacity()

	// Calculate queue usage percentages
	jouleUsage := 0.0
	if jouleCapacity > 0 {
		jouleUsage = (float64(jouleSize) / float64(jouleCapacity)) * 100
	}
	roboUsage := 0.0
	if roboCapacity > 0 {
		roboUsage = (float64(roboSize) / float64(roboCapacity)) * 100
	}

	// Get assembler metrics
	accumulatedJoules := hh.assembler.GetAccumulatedJoules()
	completedIngots := hh.assembler.GetCompletedIngotsCount()

	// Calculate progress toward next ingot (3600 threshold)
	progressPercent := 0.0
	if accumulatedJoules > 0 {
		progressPercent = (accumulatedJoules / 3600.0) * 100
		if progressPercent > 100 {
			progressPercent = 100 // Cap at 100%
		}
	}

	// Determine overall status
	status := "healthy"
	if !hh.mintClient.IsConnected() {
		status = "degraded" // NATS disconnected
	} else if jouleUsage > 90 || roboUsage > 90 {
		status = "warning" // Queues near capacity
	}

	health := HealthResponse{
		Status:    status,
		Timestamp: time.Now().UTC(),
		Queues: QueueHealth{
			JouleQueueSize:     jouleSize,
			JouleQueueCapacity: jouleCapacity,
			JouleQueueUsage:    jouleUsage,
			RoboQueueSize:      roboSize,
			RoboQueueCapacity:  roboCapacity,
			RoboQueueUsage:     roboUsage,
		},
		NATS: NATSHealth{
			Connected: hh.mintClient.IsConnected(),
			Status:    hh.mintClient.GetStatus(),
		},
		IngotAssembly: IngotAssemblyHealth{
			AccumulatedJoules: accumulatedJoules,
			CompletedIngots:   completedIngots,
			ProgressPercent:   progressPercent,
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
