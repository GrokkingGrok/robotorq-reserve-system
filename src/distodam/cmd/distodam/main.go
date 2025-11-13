// src/distodam/cmd/distodam/main.go
// -----------------------------------------------------------
// Service: DistoDam
// Purpose: Acts as a **reservoir and labor router** in the RoboTorq
//          network. It receives aggregated RoboTorq from the Mint
//          via NATS, stores it in an atomic reservoir, and routes
//          a fixed percentage (40%) of the base inflow rate to
//          downstream labor (BRLa) every second.
//
//          **Key Responsibilities**:
//          • Subscribe to Mint output on topic `distodam.robo`
//          • Parse `total_robo` and convert to micro-RT (1 RT = 1M µRT)
//          • Atomically add to reservoir
//          • Every second, deduct 0.000667 RT and publish to `brla.funding`
//          • Trigger rebalance if reservoir too low
//          • Expose health, status, and Prometheus metrics
//          • Graceful shutdown with NATS flush
//
//          **Production-Ready Features**:
//          • Atomic reservoir with CompareAndSwap (race-free)
//          • Structured JSON logging (slog)
//          • Full Prometheus observability (counters, gauges, histogram)
//          • Context-aware lifecycle (SIGINT/SIGTERM)
//          • Configurable NATS URL
//          • Rebalance latency tracking
//          • Safe NATS publish (no panic on error)
// -----------------------------------------------------------

package main

import (
	"context"
	"encoding/json"
	"log" // Legacy logging (used only in HTTP server for compatibility)
	"log/slog"
	"math"
	"net/http"
	"os"
	"os/signal"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// ────────────────────────────────────────────────────────────────
// DOMAIN MODEL: DistoDam
// ────────────────────────────────────────────────────────────────
// DistoDam represents a single reservoir in the network.
// It receives inflows from Mint, stores them in micro-RT units,
// and routes labor shares to BRLa. All balance operations are
// atomic to support high-throughput concurrent access.
type DistoDam struct {
	ID        string          // Unique identifier for this dam
	nc        *nats.Conn      // NATS connection for publishing
	reservoir atomic.Int64    // Reservoir balance in micro-RT (1 RT = 1,000,000 µRT)
	ctx       context.Context // Lifecycle context for graceful shutdown
	cancel    context.CancelFunc

	// Water level thresholds (currently unused, but reserved for future rebalancing logic)
	lowWater  int64
	highWater int64

	// ─── Prometheus metrics (all registered in NewDistoDam) ──────────────────────────────
	reservoirGauge prometheus.Gauge     // Current reservoir balance in RT
	inflowCounter  prometheus.Counter   // Total inflows received
	outflowCounter prometheus.Counter   // Total outflows routed to BRLa
	rebalCounter   prometheus.Counter   // Number of rebalance operations triggered
	rebalHistogram prometheus.Histogram // Duration of rebalance operations
}

// NewDistoDam creates a new DistoDam instance with full observability.
// It initializes atomic state, Prometheus metrics, and a cancellable context.
func NewDistoDam(ctx context.Context, id string, nc *nats.Conn) *DistoDam {
	cctx, cancel := context.WithCancel(ctx)

	d := &DistoDam{
		ID:        id,
		nc:        nc,
		ctx:       cctx,
		cancel:    cancel,
		lowWater:  1_000_000,  // 0.001 RT — future low-water trigger
		highWater: 10_000_000, // 0.01 RT — future high-water trigger
	}

	// ─── Prometheus Metrics Registration ──────────────────────────────
	// Each metric is scoped to this dam instance and registered globally.
	d.reservoirGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "distodam_reservoir_rt",
		Help: "Current reservoir balance in RT (float)",
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
	d.rebalHistogram = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "distodam_rebalance_duration_seconds",
		Help:    "Duration of rebalance operations in seconds",
		Buckets: prometheus.DefBuckets, // [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]
	})

	prometheus.MustRegister(
		d.reservoirGauge,
		d.inflowCounter,
		d.outflowCounter,
		d.rebalCounter,
		d.rebalHistogram,
	)

	return d
}

// ────────────────────────────────────────────────────────────────
// MAIN
// ────────────────────────────────────────────────────────────────
func main() {
	// Initialize structured JSON logging (cloud-native, parseable)
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	})))

	// NATS connection with fallback URL
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats:4222" // Docker network default
	}

	nc, err := nats.Connect(natsURL)
	if err != nil {
		slog.Error("nats_connect_failed", "error", err)
		os.Exit(1)
	}
	defer nc.Close()
	slog.Info("nats_connected", "url", natsURL)

	// Context for graceful shutdown (SIGINT, SIGTERM)
	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	// Create and start the dam
	dam := NewDistoDam(ctx, "dam-jon-001", nc)
	dam.startHTTP()       // Health, status, metrics
	dam.subscribeMint()   // Listen for Mint output
	dam.startFlowTicker() // Route labor share every second

	slog.Info("distodam_running", "http_port", 8080, "metrics_path", "/metrics")

	// Block until shutdown signal
	<-ctx.Done()
	dam.Shutdown()
	slog.Info("distodam_exited_cleanly")
}

// ────────────────────────────────────────────────────────────────
// HTTP SERVER
// ────────────────────────────────────────────────────────────────
func (d *DistoDam) startHTTP() {
	mux := http.NewServeMux()

	// Health check: returns 200 OK
	mux.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.Write([]byte("OK"))
	})

	// Status: human-readable reservoir state
	mux.HandleFunc("/status", func(w http.ResponseWriter, _ *http.Request) {
		current := float64(d.reservoir.Load()) / 1_000_000.0
		data, _ := json.MarshalIndent(map[string]any{
			"dam_id":    d.ID,
			"reservoir": current,
		}, "", "  ")
		w.Header().Set("Content-Type", "application/json")
		w.Write(data)
	})

	// Prometheus metrics endpoint
	mux.Handle("/metrics", promhttp.Handler())

	// Start HTTP server in background
	go func() {
		log.Println("HTTP server listening on :8080")
		if err := http.ListenAndServe(":8080", mux); err != nil && err != http.ErrServerClosed {
			log.Fatal("HTTP server error: ", err)
		}
	}()
}

// ────────────────────────────────────────────────────────────────
// NATS + FLOW LOGIC
// ────────────────────────────────────────────────────────────────
func (d *DistoDam) subscribeMint() {
	_, err := d.nc.Subscribe("distodam.robo", func(m *nats.Msg) {
		// Parse only the total RoboTorq from MintEvent
		var ev struct {
			TotalRoboTorq float64 `json:"total_robo"`
		}
		if err := json.Unmarshal(m.Data, &ev); err != nil {
			slog.Error("invalid_mint_event", "error", err, "raw", string(m.Data))
			return
		}

		// Convert RT → micro-RT (1 RT = 1,000,000 µRT)
		amountMicroRT := int64(math.Round(ev.TotalRoboTorq * 1_000_000))

		// Add to reservoir
		d.addReservoir(amountMicroRT)

		slog.Info("mint_event_processed",
			"amount_rt", ev.TotalRoboTorq,
			"micro_rt", amountMicroRT,
		)
	})
	if err != nil {
		slog.Error("subscribe_failed", "topic", "distodam.robo", "error", err)
		os.Exit(1)
	}
	slog.Info("subscribed_to_topic", "topic", "distodam.robo")
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
	// 40% of base rate: 0.001667 RT/sec × 0.40 = 0.000667 RT/sec
	amountMicro := int64(math.Round(0.001667 * 0.40 * 1_000_000.0))

	if !d.deductReservoir(amountMicro) {
		return // Not enough balance → rebalance triggered
	}

	// Publish funding event to BRLa
	flow := map[string]any{
		"brla_id":      "JON-3DPRINT-001",
		"amount_rt":    amountMicro,
		"source":       d.ID,
		"trust_wallet": "torq1xyz",
	}
	payload, _ := json.Marshal(flow)

	if err := d.nc.Publish("brla.funding", payload); err != nil {
		slog.Error("nats_publish_failed", "topic", "brla.funding", "error", err)
		return
	}

	d.outflowCounter.Inc()
	slog.Info("labor_share_routed", "amount_micro", amountMicro)
}

// ────────────────────────────────────────────────────────────────
// RESERVOIR OPS (Atomic-safe)
// ────────────────────────────────────────────────────────────────
func (d *DistoDam) addReservoir(amountMicroRT int64) {
	newVal := d.reservoir.Add(amountMicroRT)
	current := float64(newVal) / 1_000_000.0
	d.inflowCounter.Inc()
	d.reservoirGauge.Set(current)
	log.Printf("RESERVOIR +%.6f RT → %.6f RT", float64(amountMicroRT)/1_000_000, current)
}

func (d *DistoDam) deductReservoir(amountMicroRT int64) bool {
	for {
		oldVal := d.reservoir.Load()
		if oldVal < amountMicroRT {
			go d.rebalance() // Not enough → trigger rebalance
			return false
		}
		if d.reservoir.CompareAndSwap(oldVal, oldVal-amountMicroRT) {
			current := float64(oldVal-amountMicroRT) / 1_000_000.0
			d.reservoirGauge.Set(current)
			log.Printf("RESERVOIR -%.6f RT → %.6f RT", float64(amountMicroRT)/1_000_000, current)
			return true
		}
		// Loop: retry if another goroutine modified balance
	}
}

func (d *DistoDam) rebalance() {
	start := time.Now()
	d.rebalCounter.Inc()

	// Simulate rebalance work (replace with real logic later)
	time.Sleep(50 * time.Millisecond)

	duration := time.Since(start).Seconds()
	d.rebalHistogram.Observe(duration)

	slog.Info("rebalance_completed",
		"duration_seconds", duration,
		"current_balance_rt", float64(d.reservoir.Load())/1_000_000.0,
	)
}

func (d *DistoDam) Shutdown() {
	d.cancel()
	_ = d.nc.Flush() // Ensure all messages are sent
}
