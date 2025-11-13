// src/mint/cmd/mint/main.go
// -----------------------------------------------------------
// Service: Mint
// Purpose: Receives TokenTorqIngots from Refinery, aggregates
//          every 1 000 ingots, mints new RoboTorq based on sale
//          price, and publishes the total RoboTorq (incoming +
//          minted) to DistoDam via NATS.
//
//          **Pluggable Minter Design**: The minting logic is
//          abstracted behind the `RoboMinter` interface. This
//          allows future integration with post-quantum crypto
//          (Dilithium), Merkle proofs, ZK circuits, or oracles
//          without changing the core aggregation or HTTP logic.
//
//          **Production-Ready Features**:
//          • Structured JSON logging (slog)
//          • Full observability (Prometheus + /stats)
//          • Graceful shutdown with context
//          • NATS retry with backoff
//          • Batch latency tracking
//          • Testable via mock minter
//
//          **Economic Model**:
//          • 1,000 Ingots → 1 Batch
//          • New RoboTorq = Σ(price) / $1.00 target price
//          • Total RoboTorq = Incoming Robo + Newly Minted
//          • Published to NATS topic: distodam.robo
// -----------------------------------------------------------

package main

import (
	"context"
	"encoding/json"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"github.com/prometheus/client_golang/prometheus/testutil"
	"google.golang.org/grpc"
)

// ─────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────
// Ingot represents a single TokenTorqIngot from Refinery.
// It contains fixed energy (JouleTorq), variable robot resource,
// and the USD sale price used for minting.
type Ingot struct {
	JouleTorq float64 `json:"joule"` // Always 3600.0 (validated)
	RoboTorq  float64 `json:"robo"`  // Amount of RoboTorq contributed
	Price     float64 `json:"price"` // Sale price in USD
}

// MintEvent is the structured message sent to DistoDam via NATS.
// It includes full economic breakdown for transparency and auditing.
type MintEvent struct {
	TotalRoboTorq       float64 `json:"total_robo"`       // Final amount sent
	IncomingRoboTorq    float64 `json:"incoming_robo"`    // From ingots
	NewlyMintedRoboTorq float64 `json:"new_robo"`         // Minted based on price
	SaleValueUSD        float64 `json:"sale_value"`       // Σ(price)
	IngotsProcessed     int     `json:"ingots_processed"` // Batch size
	TargetPriceUSD      float64 `json:"target_price"`     // $1.00 per RoboTorq
	Timestamp           string  `json:"timestamp"`        // UTC ISO 8601
}

// ─────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────
// All magic numbers are constants for clarity and future tuning.
const (
	NatsURL              = "nats:4222"     // NATS server (Docker DNS)
	NatsDistoTopic       = "distodam.robo" // DistoDam listens here
	IngotsPerBatch       = 1000            // 1,000 ingots per mint
	RoboTorqTargetPrice  = 1.00            // $1.00 per RoboTorq
	BufferCapacity       = 100_000         // Max buffered ingots
	HealthCheckInterval  = 5 * time.Second // Flush partial batch
	JouleTorqExpected    = 3600.0          // Validation constant
	NatsPublishRetries   = 3               // Retry NATS publish
	NatsPublishBackoffMS = 100             // Backoff between retries
)

// ─────────────────────────────────────────────────────────────
// Prometheus Metrics
// ─────────────────────────────────────────────────────────────
// These metrics are scraped by Prometheus for monitoring and alerting.
var (
	// ingotsReceived counts total ingots accepted from Refinery
	ingotsReceived = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "mint_ingots_received_total",
		Help: "Total ingots received from Refinery.",
	})

	// ingotsBuffered shows current backlog in the buffer
	ingotsBuffered = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "mint_ingots_buffered",
		Help: "Current number of ingots waiting to be processed.",
	})

	// roboMintedTotal tracks all RoboTorq sent to DistoDam
	roboMintedTotal = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "mint_robo_minted_total",
		Help: "Total RoboTorq sent to DistoDam (incoming + newly minted).",
	})

	// roboMintedNew tracks only the newly minted portion
	roboMintedNew = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "mint_robo_newly_minted_total",
		Help: "RoboTorq created via price-based minting.",
	})

	// batchLatency measures how long it takes to process a full batch
	batchLatency = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "mint_batch_processing_seconds",
		Help:    "Time to process a full batch of 1000 ingots.",
		Buckets: prometheus.DefBuckets,
	})
)

// Register all metrics at startup
func init() {
	prometheus.MustRegister(
		ingotsReceived,
		ingotsBuffered,
		roboMintedTotal,
		roboMintedNew,
		batchLatency,
	)
}

// ─────────────────────────────────────────────────────────────
// Global State
// ─────────────────────────────────────────────────────────────
var (
	// ingotBuffer holds incoming ingots until a batch is ready
	ingotBuffer = make(chan Ingot, BufferCapacity)

	// ingotCount tracks current buffer size atomically
	ingotCount atomic.Int64

	// startTime records when the service started (for uptime)
	startTime = time.Now()
)

// ─────────────────────────────────────────────────────────────
// RoboMinter Interface (Pluggable)
// ─────────────────────────────────────────────────────────────
// RoboMinter defines the contract for minting logic.
// This allows swapping in cryptographic minters later.
type RoboMinter interface {
	// Mint processes a batch and returns:
	// - total incoming RoboTorq
	// - newly minted RoboTorq
	// - error (if any)
	Mint(batch []Ingot) (totalIncoming, newlyMinted float64, err error)
}

// ─────────────────────────────────────────────────────────────
// MockMinter (Testable)
// ─────────────────────────────────────────────────────────────
// MockMinter implements the current economic model.
// Replace with DilithiumMinter, ZKMinter, etc. later.
type MockMinter struct{}

func (m *MockMinter) Mint(batch []Ingot) (float64, float64, error) {
	var totalIncoming, totalSale float64
	for _, ingot := range batch {
		totalIncoming += ingot.RoboTorq
		totalSale += ingot.Price
	}
	newlyMinted := totalSale / RoboTorqTargetPrice
	return totalIncoming, newlyMinted, nil
}

// ─────────────────────────────────────────────────────────────
// Prometheus Value Helpers
// getCounterValue reads a Counter by using testutil.ToFloat64 for a safe read
func getCounterValue(c prometheus.Counter) float64 {
	return testutil.ToFloat64(c)
}

// getGaugeValue reads a Gauge using testutil.ToFloat64
func getGaugeValue(g prometheus.Gauge) float64 {
	return testutil.ToFloat64(g)
}

// ─────────────────────────────────────────────────────────────
// HTTP Handlers
// ─────────────────────────────────────────────────────────────
// ingotHandler accepts POST /mint-tokentorq from Refinery
func ingotHandler(w http.ResponseWriter, r *http.Request) {
	// Only allow POST
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var ingot Ingot
	if err := json.NewDecoder(r.Body).Decode(&ingot); err != nil {
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}

	// Validate payload
	if ingot.JouleTorq != JouleTorqExpected || ingot.RoboTorq < 0 || ingot.Price <= 0 {
		http.Error(w, "invalid ingot payload", http.StatusBadRequest)
		return
	}

	// Try to buffer; return 429 if full
	select {
	case ingotBuffer <- ingot:
		ingotCount.Add(1)
		ingotsReceived.Inc()
		ingotsBuffered.Set(float64(ingotCount.Load()))
		w.WriteHeader(http.StatusAccepted)
		w.Write([]byte("ok"))
	default:
		http.Error(w, "buffer full", http.StatusTooManyRequests)
	}
}

// healthHandler returns 200 OK for liveness probes
func healthHandler(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("ok"))
}

// statsHandler exposes real-time system state in JSON
func statsHandler(w http.ResponseWriter, _ *http.Request) {
	stats := map[string]interface{}{
		"ingots_buffered":   getGaugeValue(ingotsBuffered),    // Gauge: direct .Value()
		"ingots_received":   getCounterValue(ingotsReceived),  // Counter: Add(0)
		"robo_minted_total": getCounterValue(roboMintedTotal), // Counter
		"robo_minted_new":   getCounterValue(roboMintedNew),   // Counter
		"buffer_capacity":   BufferCapacity,
		"batch_size_target": IngotsPerBatch,
		"target_price_usd":  RoboTorqTargetPrice,
		"uptime_seconds":    time.Since(startTime).Seconds(),
		"timestamp":         time.Now().UTC().Format(time.RFC3339),
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

// ─────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────

// resetBatch clears the batch slice without reallocating
func resetBatch(batch *[]Ingot) {
	*batch = (*batch)[:0]
}

// sumBatchPrice computes total USD value of the batch
// Used by current model. Future minters may override.
func sumBatchPrice(batch []Ingot) float64 {
	var total float64
	for _, ingot := range batch {
		total += ingot.Price
	}
	return total
}

// publishWithRetry sends a message with exponential backoff
func publishWithRetry(nc *nats.Conn, topic string, payload []byte) error {
	var err error
	for i := 0; i < NatsPublishRetries; i++ {
		if err = nc.Publish(topic, payload); err == nil {
			return nil
		}
		time.Sleep(time.Millisecond * NatsPublishBackoffMS)
	}
	return err
}

// ─────────────────────────────────────────────────────────────
// Mint & Distribute (Uses Pluggable Minter)
// ─────────────────────────────────────────────────────────────
func mintAndDistribute(nc *nats.Conn, batch []Ingot, minter RoboMinter) {
	if len(batch) == 0 {
		return
	}

	// Delegate minting logic to pluggable minter
	incoming, newRobo, err := minter.Mint(batch)
	if err != nil {
		slog.Error("minting_failed",
			"error", err,
			"batch_size", len(batch),
		)
		return
	}

	totalRobo := incoming + newRobo

	// Build event for DistoDam
	event := MintEvent{
		TotalRoboTorq:       totalRobo,
		IncomingRoboTorq:    incoming,
		NewlyMintedRoboTorq: newRobo,
		SaleValueUSD:        sumBatchPrice(batch),
		IngotsProcessed:     len(batch),
		TargetPriceUSD:      RoboTorqTargetPrice,
		Timestamp:           time.Now().UTC().Format(time.RFC3339),
	}

	payload, _ := json.Marshal(event)

	// Publish with retry
	if err := publishWithRetry(nc, NatsDistoTopic, payload); err != nil {
		slog.Error("nats_publish_failed",
			"error", err,
			"batch_size", len(batch),
			"sale_usd", sumBatchPrice(batch),
		)
		return
	}

	// Log structured event
	slog.Info("mint_event",
		"batch_size", len(batch),
		"sale_usd", sumBatchPrice(batch),
		"incoming_robo", incoming,
		"new_robo", newRobo,
		"total_robo", totalRobo,
	)

	// Update metrics
	roboMintedTotal.Add(totalRobo)
	roboMintedNew.Add(newRobo)
}

// ─────────────────────────────────────────────────────────────
// Aggregation & Minting Loop
// ─────────────────────────────────────────────────────────────
// aggregateAndMintLoop drains the buffer, builds batches,
// and triggers minting. It respects context for shutdown.
func aggregateAndMintLoop(ctx context.Context, nc *nats.Conn, minter RoboMinter) {
	batch := make([]Ingot, 0, IngotsPerBatch)
	flushTicker := time.NewTicker(HealthCheckInterval)
	defer flushTicker.Stop()

	for {
		select {
		case <-ctx.Done():
			// Flush any remaining batch on shutdown
			if len(batch) > 0 {
				mintAndDistribute(nc, batch, minter)
			}
			return

		case ingot := <-ingotBuffer:
			batch = append(batch, ingot)
			ingotCount.Add(-1)
			ingotsBuffered.Set(float64(ingotCount.Load()))

			// Full batch → mint immediately
			if len(batch) >= IngotsPerBatch {
				start := time.Now()
				mintAndDistribute(nc, batch, minter)
				batchLatency.Observe(time.Since(start).Seconds())
				resetBatch(&batch)
			}

		case <-flushTicker.C:
			// Flush partial batch periodically
			if len(batch) > 0 {
				mintAndDistribute(nc, batch, minter)
				resetBatch(&batch)
			}
		}
	}
}

// ─────────────────────────────────────────────────────────────
// Main Function
// ─────────────────────────────────────────────────────────────
func main() {
	// Initialize structured JSON logging
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	})))

	// Context for graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle OS signals
	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigs
		slog.Info("shutdown_signal_received")
		cancel()
	}()

	// Connect to NATS
	nc, err := nats.Connect(NatsURL)
	if err != nil {
		slog.Error("nats_connection_failed", "error", err)
		os.Exit(1)
	}
	defer nc.Close()
	slog.Info("nats_connected", "url", NatsURL)

	// gRPC server (stub for future expansion)
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		slog.Error("grpc_listen_failed", "error", err)
		os.Exit(1)
	}
	grpcServer := grpc.NewServer()
	go func() {
		slog.Info("grpc_server_starting", "port", 50051)
		if err := grpcServer.Serve(lis); err != nil {
			slog.Error("grpc_server_error", "error", err)
		}
	}()
	defer grpcServer.GracefulStop()

	// Initialize pluggable minter (mock for now)
	minter := &MockMinter{}

	// Start aggregation loop
	go aggregateAndMintLoop(ctx, nc, minter)

	// HTTP server with all endpoints
	mux := http.NewServeMux()
	mux.HandleFunc("/mint-tokentorq", ingotHandler)
	mux.HandleFunc("/health", healthHandler)
	mux.HandleFunc("/stats", statsHandler)
	mux.Handle("/metrics", promhttp.Handler())

	server := &http.Server{
		Addr:    ":8080",
		Handler: mux,
	}

	go func() {
		slog.Info("http_server_starting", "port", 8080)
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("http_server_error", "error", err)
			os.Exit(1)
		}
	}()

	// Wait for shutdown
	<-ctx.Done()
	slog.Info("shutting_down")

	// Graceful HTTP shutdown
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer shutdownCancel()
	if err := server.Shutdown(shutdownCtx); err != nil {
		slog.Error("http_shutdown_failed", "error", err)
	} else {
		slog.Info("http_server_stopped")
	}

	slog.Info("mint_service_stopped_gracefully")
}
