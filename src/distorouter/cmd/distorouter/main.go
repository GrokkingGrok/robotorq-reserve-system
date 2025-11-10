// src/distorouter/cmd/distorouter/main.go
package main

import (
	"encoding/json" // @dev Serialize flow to JSON for NATS
	"log"           // @dev Logging to stdout (Docker logs)
	"math"          // @dev Math for rounding float to integer
	"net/http"      // @dev HTTP server for /health endpoint
	"time"          // @dev Ticker for 1-second intervals

	"github.com/nats-io/nats.go" // @dev NATS client for publishing to brla.funding
)

// @dev Entry point for DistoRouter service
// @dev Purpose: Route 40% of incoming DistoStream to labor pool via NATS
// @dev Appendix U: RT Transfers
// @dev Appendix X: BidNet → brla.funding topic
func main() {
	// @dev Connect to NATS server (internal Docker network)
	// @dev URL: nats:4222 (service name from docker-compose)
	// @dev Edge: Connection fail → fatal (container restarts)
	nc, err := nats.Connect("nats:4222")
	if err != nil {
		log.Fatal("Failed to connect to NATS: ", err)
	}
	defer nc.Close() // @dev Ensure clean shutdown

	// @dev HTTP health endpoint
	// @dev Required by docker-compose healthcheck
	// @dev Path: GET /health → "OK"
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("OK"))
	})

	// @dev Background goroutine: simulate your DistoStream
	// @dev Your rate: 6 TTP = 6 RT/hour = 0.001667 RT/sec
	// @dev 40% → labor pool = 0.0006668 RT/sec
	go func() {
		ticker := time.NewTicker(1 * time.Second) // @dev 1 Hz pulse
		defer ticker.Stop()

		for range ticker.C {
			// @dev Construct flow message
			// @dev brla_id: target BRLA
			// @dev amount_rt: 40% of your 1-second disto
			// compute 40% of the 1-second disto (in micro-units) and round to nearest integer
			amount := int64(math.Round(0.001667 * 0.40 * 1_000_000.0)) // 40% labor share

			flow := map[string]any{
				"brla_id":      "JON-3DPRINT-001",
				"amount_rt":    amount,
				"source":       "jon_wallet",
				"trust_wallet": "torq1xyz",
			}

			// @dev Serialize to JSON
			payload, err := json.Marshal(flow)
			if err != nil {
				log.Println("JSON marshal error: ", err)
				return
			}

			// @dev Publish to NATS topic
			// @dev Topic: brla.funding → Refinery subscribes
			// @dev Fire-and-forget (no ack needed)
			if err := nc.Publish("brla.funding", payload); err != nil {
				log.Println("NATS publish error: ", err)
			}
		}
	}()

	// @dev Start HTTP server
	// @dev Port: 8080 (mapped to host 8082)
	// @dev Blocks until fatal error
	log.Println("DistoRouter routing to brla.funding on :8082")
	log.Fatal(http.ListenAndServe(":8080", nil))
}
