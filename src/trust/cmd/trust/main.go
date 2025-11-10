package main

import (
	// @dev Serialize flow to JSON for NATS
	"log"      // @dev Logging to stdout (Docker logs)
	"net/http" // @dev HTTP server for /health endpoint
	"sync"     // @dev Math for rounding float to integer
	// @dev Ticker for 1-second intervals
	// @dev NATS client for publishing to brla.funding
)

type Trust struct {
	ID      string
	Wallets map[string]float64 // wallet → balance
	mu      sync.RWMutex
}

var trusts = make(map[string]*Trust)

func main() {
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("ok"))
	})

	http.HandleFunc("/pay", func(w http.ResponseWriter, r *http.Request) {
		// @dev Oracle calls /pay?trust_id=...&wallet=...&amount=...
		// @dev Add to wallet
	})

	log.Println("Trust Service running on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}
