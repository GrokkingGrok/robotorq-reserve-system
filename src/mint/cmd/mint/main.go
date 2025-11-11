// src/mint/cmd/mint/main.go
// -----------------------------------------------------------
// Service: Mint
// Purpose: Issues new RT (Real-Time Tokens) periodically and
//          publishes them to the DistoDam via NATS.
// -----------------------------------------------------------

package main

import (
	"encoding/json"
	"log"
	"math"
	"net"
	"net/http"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"google.golang.org/grpc"
)

// ─────────────────────────────────────────────────────────────
// Metrics
// ─────────────────────────────────────────────────────────────

// mint_total_rt counts the total RT minted (in RT, not μRT)
var mintTotalRT = prometheus.NewCounter(
	prometheus.CounterOpts{
		Name: "mint_total_rt",
		Help: "Total RT minted by this Mint service.",
	})

// mint_tick_rate counts each minting event (1/sec)
var mintTickCount = prometheus.NewCounter(
	prometheus.CounterOpts{
		Name: "mint_ticks_total",
		Help: "Number of minting intervals executed.",
	})

func init() {
	// Register all custom Prometheus metrics
	prometheus.MustRegister(mintTotalRT)
	prometheus.MustRegister(mintTickCount)
}

// ─────────────────────────────────────────────────────────────
// Mint Configuration
// ─────────────────────────────────────────────────────────────

const (
	NatsURL          = "nats:4222"  // NATS service (from docker-compose)
	MintTopic        = "mint.disto" // Topic DistoDam subscribes to
	MintRateRTPerSec = 0.001667     // 6 RT/hour
)

// ─────────────────────────────────────────────────────────────
// Main Entry
// ─────────────────────────────────────────────────────────────

func main() {
	log.Println("Starting Mint service...")

	// Connect to NATS (internal Docker DNS name: "nats")
	nc, err := nats.Connect(NatsURL)
	if err != nil {
		log.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()
	log.Println("Connected to NATS at", NatsURL)

	// gRPC Server (reserved for future APIs)
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("Failed to listen on gRPC port: %v", err)
	}
	s := grpc.NewServer()
	go func() {
		if err := s.Serve(lis); err != nil {
			log.Fatalf("gRPC server failed: %v", err)
		}
	}()
	log.Println("gRPC server listening on :50051")

	// Health endpoint (for Docker Compose healthcheck)
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("ok"))
	})

	// Prometheus metrics endpoint
	http.Handle("/metrics", promhttp.Handler())

	// Start minting loop
	go mintLoop(nc)

	// Start HTTP server (health + metrics)
	log.Println("Mint HTTP server on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

// ─────────────────────────────────────────────────────────────
// Minting Loop
// ─────────────────────────────────────────────────────────────

func mintLoop(nc *nats.Conn) {
	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	for range ticker.C {
		// Compute amount to mint (in μRT)
		amountMicroRT := int64(math.Round(MintRateRTPerSec * 1_000_000))

		// Prepare mint message
		mintEvent := map[string]any{
			"AmountMicroRT": amountMicroRT,
			"timestamp":     time.Now().UTC().Format(time.RFC3339),
		}

		// Serialize to JSON
		payload, err := json.Marshal(mintEvent)
		if err != nil {
			log.Printf("JSON marshal error: %v", err)
			continue
		}

		// Publish to NATS topic "mint.disto"
		if err := nc.Publish(MintTopic, payload); err != nil {
			log.Printf("Failed to publish to %s: %v", MintTopic, err)
			continue
		}

		// Update metrics
		mintTickCount.Inc()
		mintTotalRT.Add(MintRateRTPerSec)

		log.Printf("MINT: +%.6f RT (%d μRT) published to %s",
			MintRateRTPerSec, amountMicroRT, MintTopic)
	}
}
