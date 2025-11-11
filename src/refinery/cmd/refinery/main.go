// Package main implements the Refinery service, a high-throughput system that
// receives JouleTorq (energy) and RoboTorq (robotic resources), combines them into
// TokenTorqIngots using priority-based matching, and mints them via an external Mint service.
//
// The system is designed for:
// - High performance with buffered channels and dynamic worker scaling
// - Observability via Prometheus metrics
// - Resilience with retry logic and graceful shutdown
// - Fair resource allocation using max-heaps (highest value first)
package main

import (
	"bytes"
	"container/heap" // Used to implement priority queues for Joule and Robo
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"math"
	"math/rand"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"sync"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// ─────────────────────────────────────────────────────────────
// Configuration Constants
// ─────────────────────────────────────────────────────────────
// These constants define operational parameters of the Refinery.
// They are intentionally immutable to ensure consistent behavior.
const (
	HTTPPort         = ":8080"                           // HTTP server listening port
	MetricsURL       = "/metrics"                        // Prometheus exposition endpoint
	HealthURL        = "/health"                         // Health check endpoint
	MintURL          = "http://mint:8080/mint-tokentorq" // External Mint service URL
	JoulePerIngot    = 3600.0                            // Fixed JouleTorq required per ingot
	ApprovalsReq     = 3                                 // Simulated approvals before minting
	ChannelBuffer    = 10_000                            // Buffer size for input/mint channels
	HealthCacheTTL   = 2 * time.Second                   // Cache duration for health response
	MintTimeout      = 5 * time.Second                   // HTTP timeout for Mint requests
	MintMaxRetries   = 3                                 // Max retry attempts for minting
	MintRetryBackoff = 500 * time.Millisecond            // Base backoff for exponential retry
	MinMinterPool    = 2                                 // Minimum number of minting workers
	MaxMinterPool    = 10                                // Maximum number of concurrent minters
	IngotHighWater   = 50                                // Backlog threshold to scale up minters
	IngotLowWater    = 10                                // Backlog threshold to scale down minters
)

// ─────────────────────────────────────────────────────────────
// Prometheus Metrics
// ─────────────────────────────────────────────────────────────
// These metrics provide observability into system throughput, buffers, and errors.
// All are registered in init() to be scraped by Prometheus.
var (
	// refineryJouleReceived tracks total JouleTorq energy units received
	refineryJouleReceived = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_joule_received_total",
		Help: "Total JouleTorq received by the Refinery.",
	})

	// refineryRoboReceived tracks total RoboTorq units received
	refineryRoboReceived = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_robo_received_total",
		Help: "Total RoboTorq received by the Refinery.",
	})

	// refineryIngotMinted counts successful ingot mints
	refineryIngotMinted = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_ingots_minted_total",
		Help: "Total TokenTorqIngots successfully minted and sent to Mint.",
	})

	// refineryMintFailures counts failed mint attempts
	refineryMintFailures = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "refinery_mint_failures_total",
		Help: "Total failed attempts to send ingot to Mint.",
	})

	// jouleBufferGauge shows current size of JouleTorq priority queue
	jouleBufferGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_joule_buffer",
		Help: "Current number of JouleTorq in priority queue.",
	})

	// roboBufferGauge shows current size of RoboTorq priority queue
	roboBufferGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_robo_buffer",
		Help: "Current number of RoboTorq in priority queue.",
	})

	// ingotBacklogGauge shows number of ingots awaiting mint
	ingotBacklogGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_ingot_backlog",
		Help: "Current number of ingots awaiting mint.",
	})

	// minterPoolGauge shows current number of active minter workers
	minterPoolGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "refinery_minter_pool_size",
		Help: "Current number of active minter workers.",
	})
)

// init registers all Prometheus metrics at startup
func init() {
	prometheus.MustRegister(
		refineryJouleReceived,
		refineryRoboReceived,
		refineryIngotMinted,
		refineryMintFailures,
		jouleBufferGauge,
		roboBufferGauge,
		ingotBacklogGauge,
		minterPoolGauge,
	)
}

// ─────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────

// JouleTorq represents a unit of energy with a unique ID for heap tie-breaking
type JouleTorq struct {
	Amount float64 // Energy value
	id     int64   // Monotonically increasing ID for stable sort
}

// RoboTorq represents a robotic resource with amount and market price
type RoboTorq struct {
	Amount float64 // Quantity of RoboTorq
	Price  float64 // Price per unit (used for prioritization)
	id     int64   // Unique ID for heap stability
}

// Ingot is the final product: fixed Joule + variable Robo + price
type Ingot struct {
	JouleTorq float64 `json:"joule"` // Always JoulePerIngot
	RoboTorq  float64 `json:"robo"`  // From highest-priced RoboTorq
	Price     float64 `json:"price"` // Price of the matched RoboTorq
}

// ─────────────────────────────────────────────────────────────
// Priority Queue: JouleTorq (Max-Heap by Amount)
// ─────────────────────────────────────────────────────────────

// JouleHeap is a max-heap prioritizing highest Amount first.
// Uses container/heap interface.
type JouleHeap []*JouleTorq

func (h JouleHeap) Len() int { return len(h) }

// Less defines max-heap: higher Amount wins; tiebreak by lower ID (FIFO)
func (h JouleHeap) Less(i, j int) bool {
	return h[i].Amount > h[j].Amount || (h[i].Amount == h[j].Amount && h[i].id < h[j].id)
}

func (h JouleHeap) Swap(i, j int) { h[i], h[j] = h[j], h[i] }

func (h *JouleHeap) Push(x interface{}) {
	*h = append(*h, x.(*JouleTorq))
}

func (h *JouleHeap) Pop() interface{} {
	old := *h
	n := len(old)
	x := old[n-1]
	*h = old[:n-1]
	return x
}

// Consume extracts up to 'amount' of JouleTorq from the heap.
// Returns used items and true if enough was available.
// Partially consumes large items and restores heap on failure.
func (h *JouleHeap) Consume(amount float64) ([]*JouleTorq, bool) {
	var used []*JouleTorq
	var temp JouleHeap // Temporary heap to restore state on failure

	for h.Len() > 0 && amount > 0 {
		top := heap.Pop(h).(*JouleTorq)
		heap.Push(&temp, top)

		if top.Amount <= amount {
			// Fully consume this JouleTorq
			amount -= top.Amount
			used = append(used, top)
		} else {
			// Partially consume: split the item
			partial := &JouleTorq{Amount: top.Amount - amount, id: top.id}
			top.Amount = amount
			used = append(used, top)
			heap.Push(h, partial)
			amount = 0
		}
	}

	// Not enough JouleTorq: restore original heap state
	if amount > 0 {
		for temp.Len() > 0 {
			heap.Push(h, heap.Pop(&temp).(*JouleTorq))
		}
		return nil, false
	}
	return used, true
}

// ─────────────────────────────────────────────────────────────
// Priority Queue: RoboTorq (Max-Heap by Price)
// ─────────────────────────────────────────────────────────────

// RoboHeap prioritizes highest Price first for fair market matching
type RoboHeap []*RoboTorq

func (h RoboHeap) Len() int { return len(h) }

// Less: higher Price first; tiebreak by ID (stable sort)
func (h RoboHeap) Less(i, j int) bool {
	return h[i].Price > h[j].Price || (h[i].Price == h[j].Price && h[i].id < h[j].id)
}

func (h RoboHeap) Swap(i, j int)       { h[i], h[j] = h[j], h[i] }
func (h *RoboHeap) Push(x interface{}) { *h = append(*h, x.(*RoboTorq)) }

func (h *RoboHeap) Pop() interface{} {
	old := *h
	n := len(old)
	x := old[n-1]
	*h = old[:n-1]
	return x
}

// ─────────────────────────────────────────────────────────────
// Refinery Core
// ─────────────────────────────────────────────────────────────

// Refinery orchestrates the entire pipeline: ingestion, matching, minting, scaling
type Refinery struct {
	jouleCh    chan JouleTorq     // Buffered channel for incoming JouleTorq
	roboCh     chan RoboTorq      // Buffered channel for incoming RoboTorq
	ingotCh    chan Ingot         // Channel for assembled ingots awaiting mint
	client     *http.Client       // Shared HTTP client with timeout
	ctx        context.Context    // Lifecycle context
	cancel     context.CancelFunc // Cancel function for shutdown
	wg         sync.WaitGroup     // Tracks active goroutines
	jouleID    atomic.Int64       // Thread-safe ID generator for JouleTorq
	roboID     atomic.Int64       // Thread-safe ID generator for RoboTorq
	ingotCnt   atomic.Int64       // Current backlog of unminted ingots
	minterPool chan struct{}      // Semaphore channel limiting active minters

	// Health status cache with read-write lock to reduce computation
	health struct {
		mu      sync.RWMutex
		status  []byte
		lastChk time.Time
	}
}

// NewRefinery creates and initializes a Refinery with buffered channels,
// pre-allocated minter pool, and starts the processing pipeline.
func NewRefinery() *Refinery {
	ctx, cancel := context.WithCancel(context.Background())
	r := &Refinery{
		jouleCh:    make(chan JouleTorq, ChannelBuffer),
		roboCh:     make(chan RoboTorq, ChannelBuffer),
		ingotCh:    make(chan Ingot, ChannelBuffer),
		client:     &http.Client{Timeout: MintTimeout},
		ctx:        ctx,
		cancel:     cancel,
		minterPool: make(chan struct{}, MaxMinterPool),
	}

	// Pre-allocate minimum number of minter slots
	for i := 0; i < MinMinterPool; i++ {
		r.minterPool <- struct{}{}
	}

	// Start core processing loop and initial minter workers
	r.startPipeline()
	return r
}

// ─────────────────────────────────────────────────────────────
// Core Processing Pipeline
// ─────────────────────────────────────────────────────────────

// startPipeline runs the main loop: drains inputs → builds ingots → scales workers
func (r *Refinery) startPipeline() {
	r.wg.Add(1)
	go func() {
		defer r.wg.Done()

		// Initialize priority queues
		jouleHeap := &JouleHeap{}
		heap.Init(jouleHeap)
		roboHeap := &RoboHeap{}
		heap.Init(roboHeap)

		// Periodic ticker triggers channel draining and ingot assembly
		ticker := time.NewTicker(100 * time.Millisecond)
		defer ticker.Stop()

		for {
			select {
			case <-r.ctx.Done():
				return // Graceful shutdown
			case <-ticker.C:
				// ── Drain input channels into priority queues ──
				for {
					select {
					case j := <-r.jouleCh:
						j.id = r.jouleID.Add(1)
						heap.Push(jouleHeap, &j)
					case rt := <-r.roboCh:
						rt.id = r.roboID.Add(1)
						heap.Push(roboHeap, &rt)
					default:
						goto DrainDone // No more immediate input
					}
				}
			DrainDone:

				// Update observability gauges
				jouleBufferGauge.Set(float64(jouleHeap.Len()))
				roboBufferGauge.Set(float64(roboHeap.Len()))

				// ── Assemble ingots: match highest-price Robo with enough Joule ──
				for roboHeap.Len() > 0 {
					if _, ok := jouleHeap.Consume(JoulePerIngot); !ok {
						break // Not enough JouleTorq
					}
					rt := heap.Pop(roboHeap).(*RoboTorq)
					roboBufferGauge.Set(float64(roboHeap.Len()))

					ingot := Ingot{
						JouleTorq: JoulePerIngot,
						RoboTorq:  rt.Amount,
						Price:     rt.Price,
					}
					r.ingotCnt.Add(1)
					ingotBacklogGauge.Set(float64(r.ingotCnt.Load()))

					// Send to minting queue (non-blocking if context canceled)
					select {
					case r.ingotCh <- ingot:
					case <-r.ctx.Done():
						return
					}
				}

				// Dynamically scale minter pool based on backlog
				r.scaleMinters()
			}
		}
	}()

	// Spawn initial minter workers
	for i := 0; i < MinMinterPool; i++ {
		r.spawnMinter()
	}
}

// ─────────────────────────────────────────────────────────────
// Dynamic Minter Pool Scaling
// ─────────────────────────────────────────────────────────────

// scaleMinters adjusts the number of active minter workers based on ingot backlog
func (r *Refinery) scaleMinters() {
	backlog := int(r.ingotCnt.Load())
	current := len(r.minterPool)

	// Scale up: high backlog and under max capacity
	if backlog > IngotHighWater && current < MaxMinterPool {
		select {
		case r.minterPool <- struct{}{}:
			r.spawnMinter()
		default:
			// Already at max
		}
	} else if backlog < IngotLowWater && current > MinMinterPool {
		// Scale down: low backlog and above minimum
		select {
		case <-r.minterPool:
			// Worker will exit on next pool drain
		default:
		}
	}
	minterPoolGauge.Set(float64(len(r.minterPool)))
}

// spawnMinter starts a worker that processes ingots from ingotCh until terminated
func (r *Refinery) spawnMinter() {
	r.wg.Add(1)
	go func() {
		defer r.wg.Done()
		for {
			select {
			case <-r.ctx.Done():
				return
			case ingot := <-r.ingotCh:
				r.ingotCnt.Add(-1)
				ingotBacklogGauge.Set(float64(r.ingotCnt.Load()))
				r.approvalWorkflow(r.ctx, &ingot)
			case <-r.minterPool:
				// Pool slot reclaimed → exit worker
				return
			}
		}
	}()
}

// ─────────────────────────────────────────────────────────────
// Input Ingestion
// ─────────────────────────────────────────────────────────────

// addJoule attempts to buffer a JouleTorq; returns error if buffer full or shutting down
func (r *Refinery) addJoule(j JouleTorq) error {
	select {
	case r.jouleCh <- j:
		refineryJouleReceived.Add(j.Amount)
		return nil
	case <-r.ctx.Done():
		return fmt.Errorf("refinery shutting down")
	default:
		return fmt.Errorf("joule buffer full")
	}
}

// addRobo buffers a RoboTorq; updates metrics and handles backpressure
func (r *Refinery) addRobo(rt RoboTorq) error {
	select {
	case r.roboCh <- rt:
		refineryRoboReceived.Add(rt.Amount)
		return nil
	case <-r.ctx.Done():
		return fmt.Errorf("refinery shutting down")
	default:
		return fmt.Errorf("robo buffer full")
	}
}

// ─────────────────────────────────────────────────────────────
// Approval Workflow & Minting
// ─────────────────────────────────────────────────────────────

// approvalWorkflow simulates a multi-approval process before minting
func (r *Refinery) approvalWorkflow(ctx context.Context, ingot *Ingot) {
	slog.Info("approval workflow started", "ingot", ingot)

	// Simulate ApprovalsReq number of approvals with random delays
	for i := 0; i < ApprovalsReq; i++ {
		select {
		case <-ctx.Done():
			slog.Warn("workflow canceled", "ingot", ingot)
			return
		case <-time.After(time.Duration(rand.Intn(1000)+500) * time.Millisecond):
			// Approval granted
		}
	}

	// Attempt mint with retry
	if err := r.sendToMintWithRetry(ctx, ingot); err != nil {
		refineryMintFailures.Inc()
		slog.Error("failed to mint ingot", "error", err, "ingot", ingot)
	} else {
		refineryIngotMinted.Inc()
		slog.Info("ingot minted successfully", "ingot", ingot)
	}
}

// sendToMintWithRetry sends ingot to Mint service with exponential backoff
func (r *Refinery) sendToMintWithRetry(ctx context.Context, ingot *Ingot) error {
	data, _ := json.Marshal(ingot)

	for attempt := 0; attempt <= MintMaxRetries; attempt++ {
		req, _ := http.NewRequestWithContext(ctx, http.MethodPost, MintURL, bytes.NewReader(data))
		req.Header.Set("Content-Type", "application/json")

		resp, err := r.client.Do(req)
		if err == nil && resp != nil && resp.StatusCode < 300 {
			resp.Body.Close()
			return nil
		}
		if resp != nil {
			resp.Body.Close()
		}

		// Retry with exponential backoff unless final attempt
		if attempt < MintMaxRetries {
			backoff := time.Duration(math.Pow(2, float64(attempt))) * MintRetryBackoff
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(backoff):
			}
		}
	}
	return fmt.Errorf("exhausted retries")
}

// ─────────────────────────────────────────────────────────────
// HTTP Handlers
// ─────────────────────────────────────────────────────────────

// parseFloatParam extracts and validates a required positive float query param
func parseFloatParam(req *http.Request, key string) (float64, error) {
	s := req.URL.Query().Get(key)
	if s == "" {
		return 0, fmt.Errorf("%s required", key)
	}
	f, err := strconv.ParseFloat(s, 64)
	if err != nil || f <= 0 {
		return 0, fmt.Errorf("invalid %s", key)
	}
	return f, nil
}

// healthHandler returns cached health status; computes fresh if stale
func (r *Refinery) healthHandler(w http.ResponseWriter, req *http.Request) {
	// Try to serve from cache
	r.health.mu.RLock()
	if time.Since(r.health.lastChk) < HealthCacheTTL {
		w.Header().Set("Content-Type", "application/json")
		w.Write(r.health.status)
		r.health.mu.RUnlock()
		return
	}
	r.health.mu.RUnlock()

	// Compute fresh health
	status := "ok"
	totalLoad := len(r.jouleCh) + len(r.roboCh) + int(r.ingotCnt.Load())
	if totalLoad > ChannelBuffer/2 {
		status = "degraded"
	}

	healthJSON, _ := json.Marshal(map[string]interface{}{
		"status":       status,
		"jouleBuffer":  len(r.jouleCh),
		"roboBuffer":   len(r.roboCh),
		"ingotBacklog": r.ingotCnt.Load(),
		"minterPool":   len(r.minterPool),
	})

	// Cache result
	r.health.mu.Lock()
	r.health.status = healthJSON
	r.health.lastChk = time.Now()
	r.health.mu.Unlock()

	// Respond
	w.Header().Set("Content-Type", "application/json")
	if status != "ok" {
		w.WriteHeader(http.StatusServiceUnavailable)
	}
	w.Write(healthJSON)
}

// receiveJouleHandler accepts JouleTorq via HTTP GET ?amount=X
func (r *Refinery) receiveJouleHandler(w http.ResponseWriter, req *http.Request) {
	amt, err := parseFloatParam(req, "amount")
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := r.addJoule(JouleTorq{Amount: amt}); err != nil {
		http.Error(w, err.Error(), http.StatusTooManyRequests)
		return
	}
	fmt.Fprint(w, "Joule received\n")
}

// receiveRoboHandler accepts RoboTorq via HTTP GET ?amount=X&price=Y
func (r *Refinery) receiveRoboHandler(w http.ResponseWriter, req *http.Request) {
	amt, err := parseFloatParam(req, "amount")
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	price, err := parseFloatParam(req, "price")
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	if err := r.addRobo(RoboTorq{Amount: amt, Price: price}); err != nil {
		http.Error(w, err.Error(), http.StatusTooManyRequests)
		return
	}
	fmt.Fprint(w, "Robo received\n")
}

// ─────────────────────────────────────────────────────────────
// Main Function & Graceful Shutdown
// ─────────────────────────────────────────────────────────────
func main() {
	// Configure structured JSON logging
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo})))

	refinery := NewRefinery()
	slog.Info("Refinery starting", "port", HTTPPort)

	// Set up HTTP routes
	mux := http.NewServeMux()
	mux.Handle(MetricsURL, promhttp.Handler())
	mux.HandleFunc(HealthURL, refinery.healthHandler)
	mux.HandleFunc("/receive-joule", refinery.receiveJouleHandler)
	mux.HandleFunc("/receive-robo", refinery.receiveRoboHandler)

	// Start HTTP server
	server := &http.Server{Addr: HTTPPort, Handler: mux}
	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server stopped", "error", err)
			os.Exit(1)
		}
	}()

	// Wait for termination signal
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop

	// Initiate graceful shutdown
	slog.Info("shutting down refinery...")
	refinery.cancel()
	refinery.wg.Wait() // Wait for all workers to finish

	// Shutdown HTTP server with timeout
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := server.Shutdown(ctx); err != nil {
		slog.Error("server forced to shutdown", "error", err)
	} else {
		slog.Info("server stopped gracefully")
	}
}
