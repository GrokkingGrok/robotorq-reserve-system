// src/distodam/cmd/distodam/main.go
package main

import (
	"context"
	"encoding/json"
	"log"
	"math"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

//
// ────────────────────────────────────────────────────────────────
//   DOMAIN MODEL: DistoDam
// ────────────────────────────────────────────────────────────────
//

type DistoDam struct {
	ID        string
	nc        *nats.Conn
	reservoir int64
	mu        sync.RWMutex
	ctx       context.Context
	cancel    context.CancelFunc

	lowWater  int64
	highWater int64

	// ─── Prometheus metrics ──────────────────────────────
	reservoirGauge prometheus.Gauge
	inflowCounter  prometheus.Counter
	outflowCounter prometheus.Counter
	rebalCounter   prometheus.Counter
}

// NewDistoDam creates a new dam instance with Prometheus instrumentation.
func NewDistoDam(ctx context.Context, id string, nc *nats.Conn) *DistoDam {
	cctx, cancel := context.WithCancel(ctx)

	d := &DistoDam{
		ID:        id,
		nc:        nc,
		reservoir: 0,
		ctx:       cctx,
		cancel:    cancel,
		lowWater:  1_000_000,
		highWater: 10_000_000,
	}

	// Register Prometheus metrics
	d.reservoirGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "distodam_reservoir_rt",
		Help: "Current reservoir balance in RT",
	})
	d.inflowCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "distodam_inflows_total",
		Help: "Total number of inflows added to the reservoir",
	})
	d.outflowCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "distodam_outflows_total",
		Help: "Total number of outflows (labor routing, etc.)",
	})
	d.rebalCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "distodam_rebalances_total",
		Help: "Number of rebalance operations triggered",
	})

	prometheus.MustRegister(
		d.reservoirGauge,
		d.inflowCounter,
		d.outflowCounter,
		d.rebalCounter,
	)

	return d
}

//
// ────────────────────────────────────────────────────────────────
//   MAIN
// ────────────────────────────────────────────────────────────────
//

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

	dam := NewDistoDam(ctx, "dam-jon-001", nc)

	dam.startHTTP()
	dam.subscribeMint()
	dam.startFlowTicker()

	log.Println("✅ DistoDam running on :8080 (metrics at /metrics)")

	<-ctx.Done()
	dam.Shutdown()
	log.Println("✅ DistoDam exited cleanly.")
}

//
// ────────────────────────────────────────────────────────────────
//   HTTP SERVER
// ────────────────────────────────────────────────────────────────
//

func (d *DistoDam) startHTTP() {
	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.Write([]byte("OK"))
	})

	mux.HandleFunc("/status", func(w http.ResponseWriter, _ *http.Request) {
		d.mu.RLock()
		data, _ := json.MarshalIndent(map[string]any{
			"dam_id":    d.ID,
			"reservoir": float64(d.reservoir) / 1_000_000.0,
		}, "", "  ")
		d.mu.RUnlock()
		w.Header().Set("Content-Type", "application/json")
		w.Write(data)
	})

	// Prometheus metrics endpoint
	mux.Handle("/metrics", promhttp.Handler())

	go func() {
		log.Println("HTTP server listening on :8080")
		if err := http.ListenAndServe(":8080", mux); err != nil && err != http.ErrServerClosed {
			log.Fatal("HTTP server error: ", err)
		}
	}()
}

//
// ────────────────────────────────────────────────────────────────
//   NATS + FLOW LOGIC
// ────────────────────────────────────────────────────────────────
//

func (d *DistoDam) subscribeMint() {
	_, err := d.nc.Subscribe("mint.disto", func(m *nats.Msg) {
		var inflow struct{ AmountMicroRT int64 }
		if err := json.Unmarshal(m.Data, &inflow); err != nil {
			log.Printf("Invalid mint.disto payload: %v", err)
			return
		}
		d.addReservoir(inflow.AmountMicroRT)
	})
	if err != nil {
		log.Fatal("Failed to subscribe to mint.disto:", err)
	}
}

func (d *DistoDam) startFlowTicker() {
	ticker := time.NewTicker(1 * time.Second)
	go func() {
		defer ticker.Stop()
		for {
			select {
			case <-d.ctx.Done():
				return
			case <-ticker.C:
				d.routeLaborShare()
			}
		}
	}()
}

func (d *DistoDam) routeLaborShare() {
	amountMicro := int64(math.Round(0.001667 * 0.40 * 1_000_000.0))
	if !d.deductReservoir(amountMicro) {
		return
	}

	flow := map[string]any{
		"brla_id":      "JON-3DPRINT-001",
		"amount_rt":    amountMicro,
		"source":       d.ID,
		"trust_wallet": "torq1xyz",
	}
	payload, _ := json.Marshal(flow)
	d.nc.Publish("brla.funding", payload)
	d.outflowCounter.Inc()
}

//
// ────────────────────────────────────────────────────────────────
//   RESERVOIR OPS
// ────────────────────────────────────────────────────────────────
//

func (d *DistoDam) addReservoir(amountMicroRT int64) {
	d.mu.Lock()
	d.reservoir += amountMicroRT
	current := float64(d.reservoir) / 1_000_000.0
	d.mu.Unlock()
	d.inflowCounter.Inc()
	d.reservoirGauge.Set(current)
	log.Printf("💧 RESERVOIR +%.6f RT → %.6f RT", float64(amountMicroRT)/1_000_000, current)
}

func (d *DistoDam) deductReservoir(amountMicroRT int64) bool {
	d.mu.Lock()
	if d.reservoir < amountMicroRT {
		d.mu.Unlock()
		go d.rebalance()
		return false
	}
	d.reservoir -= amountMicroRT
	current := float64(d.reservoir) / 1_000_000.0
	d.mu.Unlock()
	d.reservoirGauge.Set(current)
	log.Printf("💸 RESERVOIR -%.6f RT → %.6f RT", float64(amountMicroRT)/1_000_000, current)
	return true
}

func (d *DistoDam) rebalance() {
	d.rebalCounter.Inc()
	log.Println("🔄 Rebalance triggered (mock)")
}

func (d *DistoDam) Shutdown() {
	d.cancel()
	d.nc.Flush()
}
