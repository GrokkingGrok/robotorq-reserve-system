package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// ─────────────────────────────────────────────────────────────
// Metrics
// ─────────────────────────────────────────────────────────────

var (
	refineryJouleReceived = prometheus.NewCounter(
		prometheus.CounterOpts{
			Name: "refinery_joule_received_total",
			Help: "Total JouleTorq received by the Refinery.",
		},
	)
	refineryRoboReceived = prometheus.NewCounter(
		prometheus.CounterOpts{
			Name: "refinery_robo_received_total",
			Help: "Total RoboTorq received by the Refinery.",
		},
	)
	refineryIngotMinted = prometheus.NewCounter(
		prometheus.CounterOpts{
			Name: "refinery_ingots_minted_total",
			Help: "Total TokenTorqIngots minted and sent to Mint.",
		},
	)
	lastHealthCheck     time.Time
	cachedHealthStatus  = []byte("ok")
	healthCacheDuration = 2 * time.Second
	healthMu            sync.RWMutex
)

func init() {
	prometheus.MustRegister(refineryJouleReceived, refineryRoboReceived, refineryIngotMinted)
}

// ─────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────

const (
	HTTPPort      = ":8080"
	MetricsURL    = "/metrics"
	HealthURL     = "/health"
	MintURL       = "http://mint:8080/mint-tokentorq"
	JoulePerIngot = 3600.0
	ApprovalsReq  = 3
)

// ─────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────

type JouleTorq struct{ Amount float64 }
type RoboTorq struct{ Amount float64 }
type Ingot struct {
	JouleTorq float64 `json:"joule"`
	RoboTorq  float64 `json:"robo"`
}

type Refinery struct {
	mu         sync.Mutex
	jouleQueue []JouleTorq
	roboQueue  []RoboTorq
	// Health cache (optional)
	healthCache struct {
		mu      sync.RWMutex
		status  string
		lastChk time.Time
	}
}

var refinery = Refinery{
	jouleQueue: []JouleTorq{},
	roboQueue:  []RoboTorq{},
}

// ─────────────────────────────────────────────────────────────
// Refinery Methods
// ─────────────────────────────────────────────────────────────

func (r *Refinery) addJoule(j JouleTorq) {
	r.mu.Lock()
	r.jouleQueue = append(r.jouleQueue, j)
	refineryJouleReceived.Add(j.Amount)
	r.mu.Unlock()

	log.Printf("Received JouleTorq: %.2f", j.Amount)
	r.tryCreateIngots()
}

func (r *Refinery) addRobo(rt RoboTorq) {
	r.mu.Lock()
	r.roboQueue = append(r.roboQueue, rt)
	refineryRoboReceived.Add(rt.Amount)
	r.mu.Unlock()

	log.Printf("Received RoboTorq: %.2f", rt.Amount)
	r.tryCreateIngots()
}

// Attempt to create **all possible ingots** safely in parallel
func (r *Refinery) tryCreateIngots() {
	r.mu.Lock()
	var ingots []*Ingot

	for {
		var accumulated float64
		var usedJoule []int

		// Collect enough JouleTorq
		for i, j := range r.jouleQueue {
			if accumulated < JoulePerIngot {
				remaining := JoulePerIngot - accumulated
				if j.Amount <= remaining {
					accumulated += j.Amount
					usedJoule = append(usedJoule, i)
				} else {
					accumulated += remaining
					r.jouleQueue[i].Amount -= remaining
					break
				}
			}
		}

		if accumulated < JoulePerIngot || len(r.roboQueue) == 0 {
			break
		}

		// Remove used JouleTorq
		newQueue := []JouleTorq{}
		for i, j := range r.jouleQueue {
			skip := false
			for _, idx := range usedJoule {
				if i == idx {
					skip = true
					break
				}
			}
			if !skip {
				newQueue = append(newQueue, j)
			}
		}
		r.jouleQueue = newQueue

		// Consume one RoboTorq
		rt := r.roboQueue[0]
		r.roboQueue = r.roboQueue[1:]

		ingots = append(ingots, &Ingot{
			JouleTorq: JoulePerIngot,
			RoboTorq:  rt.Amount,
		})
	}

	r.mu.Unlock()

	// Launch approval workflows **outside the lock**
	for _, ingot := range ingots {
		go r.approvalWorkflow(ingot)
	}
}

func (r *Refinery) approvalWorkflow(ingot *Ingot) {
	log.Printf("Starting approval workflow: %.2f JouleTorq + %.2f RoboTorq", ingot.JouleTorq, ingot.RoboTorq)

	for i := 1; i <= ApprovalsReq; i++ {
		time.Sleep(time.Duration(rand.Intn(1000)+500) * time.Millisecond)
		log.Printf("Approval %d/%d for ingot", i, ApprovalsReq)
	}

	r.sendToMint(ingot)
}

// Send ingot to Mint using **JSON POST**
func (r *Refinery) sendToMint(ingot *Ingot) {
	data, err := json.Marshal(ingot)
	if err != nil {
		log.Printf("Failed to marshal ingot: %v", err)
		return
	}

	req, err := http.NewRequestWithContext(context.Background(), http.MethodPost, MintURL, bytes.NewReader(data))
	if err != nil {
		log.Printf("Failed to create request: %v", err)
		return
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		log.Printf("Failed to send ingot to Mint: %v", err)
		return
	}
	defer resp.Body.Close()

	refineryIngotMinted.Inc()
	log.Printf("Ingot sent to Mint: %.2f JouleTorq + %.2f RoboTorq", ingot.JouleTorq, ingot.RoboTorq)
}

// ─────────────────────────────────────────────────────────────
// HTTP Handlers
// ─────────────────────────────────────────────────────────────

func main() {
	log.Println("Starting Refinery service...")

	// Health endpoint
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		healthMu.RLock()
		if time.Since(lastHealthCheck) < healthCacheDuration {
			w.Write(cachedHealthStatus)
			healthMu.RUnlock()
			return
		}
		healthMu.RUnlock()

		// Simulate Mint ping or any expensive check here
		status := []byte("ok")

		healthMu.Lock()
		cachedHealthStatus = status
		lastHealthCheck = time.Now()
		healthMu.Unlock()

		w.Write(status)
	})

	// Prometheus metrics
	http.Handle(MetricsURL, promhttp.Handler())

	// Receive Joule
	http.HandleFunc("/receive-joule", func(w http.ResponseWriter, r *http.Request) {
		amount := r.URL.Query().Get("amount")
		var a float64
		fmt.Sscanf(amount, "%f", &a)
		refinery.addJoule(JouleTorq{Amount: a})
		w.Write([]byte(fmt.Sprintf("Joule %.2f received", a)))
	})

	// Receive Robo
	http.HandleFunc("/receive-robo", func(w http.ResponseWriter, r *http.Request) {
		amount := r.URL.Query().Get("amount")
		var a float64
		fmt.Sscanf(amount, "%f", &a)
		refinery.addRobo(RoboTorq{Amount: a})
		w.Write([]byte(fmt.Sprintf("Robo %.2f received", a)))
	})

	log.Printf("Refinery HTTP server running on %s", HTTPPort)
	log.Fatal(http.ListenAndServe(HTTPPort, nil))
}
