package httpapi

import (
	"encoding/json"
	"io"
	"net/http"

	"go.uber.org/zap"

	"b2b/trust/internal/metrics"
	"b2b/trust/internal/opportunity"
)

// HTTPServer holds the dependencies required for the API.
type HTTPServer struct {
	Logger     *zap.Logger
	Metrics    *metrics.Metrics
	TickerChan chan<- *opportunity.Opportunity // write-only
}

// New creates a configured HTTPServer.
func New(logger *zap.Logger, m *metrics.Metrics, tickerInput chan<- *opportunity.Opportunity) *HTTPServer {
	return &HTTPServer{
		Logger:     logger,
		Metrics:    m,
		TickerChan: tickerInput,
	}
}

// RegisterRoutes attaches all handlers to a mux.
func (h *HTTPServer) RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("/opportunities", h.handlePostOpportunities)
	mux.HandleFunc("/metrics", h.handleMetrics)
	mux.HandleFunc("/health", h.handleHealth)
}

/* ------------------------------
   POST /opportunities
   Accepts an array of opportunities.
------------------------------ */

func (h *HTTPServer) handlePostOpportunities(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "use POST", http.StatusMethodNotAllowed)
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "failed to read body", http.StatusBadRequest)
		return
	}

	var opps []*opportunity.Opportunity
	if err := json.Unmarshal(body, &opps); err != nil {
		http.Error(w, "invalid JSON", http.StatusBadRequest)
		return
	}

	// Feed each into the Trust pipeline
	for _, op := range opps {
		h.TickerChan <- op
		h.Metrics.IncSubmitted()
	}

	h.Logger.Info("Opportunities received",
		zap.Int("count", len(opps)),
	)

	w.WriteHeader(http.StatusAccepted)
	w.Write([]byte(`{"status":"queued"}`))
}

/* ------------------------------
   GET /metrics
------------------------------ */

func (h *HTTPServer) handleMetrics(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "use GET", http.StatusMethodNotAllowed)
		return
	}

	out := map[string]uint64{
		"submitted":          h.Metrics.GetSubmitted(),
		"appraised":          h.Metrics.GetAppraised(),
		"funds_synced":       h.Metrics.GetFundsSynced(),
		"executions":         h.Metrics.GetExecutions(),
		"contracts_created":  h.Metrics.GetContractsCreated(),
		"contracts_funded":   h.Metrics.GetContractsFunded(),
		"contracts_executed": h.Metrics.GetContractsExecuted(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(out)
}

/* ------------------------------
   GET /health
------------------------------ */

func (h *HTTPServer) handleHealth(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("ok"))
}
