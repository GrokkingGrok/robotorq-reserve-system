// src/refinery/cmd/refinery/main.go
package main

import (
	//"encoding/json" // @dev Parse JSON from NATS messages
	"log"      // @dev Structured logging to stdout (Docker)
	"net/http" // @dev HTTP server for /health endpoint

	"github.com/nats-io/nats.go" // @dev NATS client for subscribing to brla.funding
)

// @dev Entry point for Refinery service
// @dev Purpose: Listen to brla.funding, aggregate, mint BRLA
// @dev Appendix X: BidNet → BRLA Minting
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
	// @dev Path: GET /health → "ok"
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("ok"))
	})

	if err != nil {
		log.Fatal("Failed:", err)
	}

	// @dev Start HTTP server
	// @dev Port: 8080 (mapped to host 8081)
	// @dev Blocks until fatal error

	log.Fatal(http.ListenAndServe(":8080", nil))
}
