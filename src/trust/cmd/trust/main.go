// src/trust/cmd/trust/main.go
package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"sync"
	"syscall"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

type Trust struct {
	ID      string
	Balance float64
	mu      sync.RWMutex
}

type TrustService struct {
	nc     *nats.Conn
	trusts map[string]*Trust
	mu     sync.RWMutex
	ctx    context.Context
	cancel context.CancelFunc

	// ─── Prometheus metrics ──────────────────────────────
	inflowCounter  *prometheus.CounterVec
	outflowCounter *prometheus.CounterVec
	balanceGauge   *prometheus.GaugeVec
}

func NewTrustService(ctx context.Context, nc *nats.Conn) *TrustService {
	cctx, cancel := context.WithCancel(ctx)

	s := &TrustService{
		nc:     nc,
		trusts: make(map[string]*Trust),
		ctx:    cctx,
		cancel: cancel,
	}

	s.inflowCounter = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "trust_inflows_total",
		Help: "Number of inflow operations by source",
	}, []string{"source"})

	s.outflowCounter = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "trust_outflows_total",
		Help: "Number of outflow operations by destination",
	}, []string{"dest"})

	s.balanceGauge = prometheus.NewGaugeVec(prometheus.GaugeOpts{
		Name: "trust_balance_rt",
		Help: "Current RT balance per BRLA",
	}, []string{"brla_id"})

	prometheus.MustRegister(s.inflowCounter, s.outflowCounter, s.balanceGauge)
	return s
}

func main() {
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats:4222"
	}
	nc, err := nats.Connect(natsURL)
	if err != nil {
		log.Fatal("Failed to connect to NATS: ", err)
	}
	defer nc.Close()

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	service := NewTrustService(ctx, nc)
	service.subscribeFunding()
	service.startHTTP()

	log.Println("✅ Trust Service running on :8080 (metrics at /metrics)")

	<-ctx.Done()
	service.Shutdown()
	log.Println("✅ Trust Service exited cleanly.")
}

//
// ────────────────────────────────────────────────────────────────
//   NATS & HTTP ROUTES
// ────────────────────────────────────────────────────────────────
//

type brlaFundFlow struct {
	BrlaID        string `json:"brla_id"`
	AmountMicroRT int64  `json:"amount_rt"`
	Source        string `json:"source"`
	TrustWallet   string `json:"trust_wallet"`
}

func (ts *TrustService) subscribeFunding() {
	_, err := ts.nc.Subscribe("brla.funding", func(m *nats.Msg) {
		var flow brlaFundFlow
		if err := json.Unmarshal(m.Data, &flow); err != nil {
			log.Printf("Invalid brla.funding payload: %v", err)
			return
		}
		amountRT := float64(flow.AmountMicroRT) / 1_000_000.0
		ts.addInflow(flow.BrlaID, "distodam", amountRT)
	})
	if err != nil {
		log.Fatal("Failed to subscribe to brla.funding:", err)
	}
}

func (ts *TrustService) startHTTP() {
	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.Write([]byte("ok"))
	})

	mux.HandleFunc("/status", func(w http.ResponseWriter, _ *http.Request) {
		ts.mu.RLock()
		snapshot := make(map[string]float64)
		for id, t := range ts.trusts {
			t.mu.RLock()
			snapshot[id] = t.Balance
			t.mu.RUnlock()
		}
		ts.mu.RUnlock()
		data, _ := json.MarshalIndent(snapshot, "", "  ")
		w.Header().Set("Content-Type", "application/json")
		w.Write(data)
	})

	// ─── Prometheus metrics ─────────────────────────────
	mux.Handle("/metrics", promhttp.Handler())

	// ─── Simplified inflow/outflow endpoints ────────────
	mux.HandleFunc("/invest", func(w http.ResponseWriter, r *http.Request) {
		ts.addInflow(r.URL.Query().Get("brla_id"), "investor", parseFloat(r.URL.Query().Get("amount")))
		w.Write([]byte("Invested"))
	})
	mux.HandleFunc("/retainer", func(w http.ResponseWriter, r *http.Request) {
		ts.addInflow(r.URL.Query().Get("brla_id"), "builder", parseFloat(r.URL.Query().Get("amount")))
		w.Write([]byte("Retainer paid"))
	})
	mux.HandleFunc("/sale", func(w http.ResponseWriter, r *http.Request) {
		ts.addInflow(r.URL.Query().Get("brla_id"), "customer", parseFloat(r.URL.Query().Get("amount")))
		w.Write([]byte("Sale recorded"))
	})
	mux.HandleFunc("/pay-investor", func(w http.ResponseWriter, r *http.Request) {
		ts.payOutflow(r.URL.Query().Get("brla_id"), "investor", parseFloat(r.URL.Query().Get("amount")))
		w.Write([]byte("Paid investor"))
	})

	go func() {
		if err := http.ListenAndServe(":8080", mux); err != nil && err != http.ErrServerClosed {
			log.Fatal("HTTP server error: ", err)
		}
	}()
}

//
// ────────────────────────────────────────────────────────────────
//   TRUST OPERATIONS
// ────────────────────────────────────────────────────────────────
//

func (ts *TrustService) addInflow(brlaID, source string, amountRT float64) {
	if brlaID == "" {
		log.Println("Missing BRLA ID inflow")
		return
	}
	t := ts.getOrCreateTrust(brlaID)
	t.mu.Lock()
	t.Balance += amountRT
	newBal := t.Balance
	t.mu.Unlock()

	ts.inflowCounter.WithLabelValues(source).Inc()
	ts.balanceGauge.WithLabelValues(brlaID).Set(newBal)
	log.Printf("💰 INFLOW: %s → %s: +%.6f RT (balance=%.6f)", source, brlaID, amountRT, newBal)
}

func (ts *TrustService) payOutflow(brlaID, dest string, amountRT float64) {
	if brlaID == "" {
		log.Println("Missing BRLA ID outflow")
		return
	}
	t := ts.getOrCreateTrust(brlaID)
	t.mu.Lock()
	if t.Balance < amountRT {
		log.Printf("⚠️ INSUFFICIENT: %s balance %.6f < %.6f", brlaID, t.Balance, amountRT)
		t.mu.Unlock()
		return
	}
	t.Balance -= amountRT
	newBal := t.Balance
	t.mu.Unlock()

	ts.outflowCounter.WithLabelValues(dest).Inc()
	ts.balanceGauge.WithLabelValues(brlaID).Set(newBal)
	log.Printf("💸 OUTFLOW: %s → %s: -%.6f RT (balance=%.6f)", brlaID, dest, amountRT, newBal)
}

func (ts *TrustService) getOrCreateTrust(brlaID string) *Trust {
	ts.mu.Lock()
	defer ts.mu.Unlock()
	t, ok := ts.trusts[brlaID]
	if !ok {
		t = &Trust{ID: brlaID}
		ts.trusts[brlaID] = t
	}
	return t
}

func parseFloat(s string) float64 {
	v, _ := strconv.ParseFloat(s, 64)
	return v
}

func (ts *TrustService) Shutdown() {
	ts.cancel()
	ts.nc.Flush()
}
