package main

import (
	// @dev Serialize flow to JSON for NATS
	"encoding/json"
	"log"      // @dev Logging to stdout (Docker logs)
	"net/http" // @dev HTTP server for /health endpoint
	"sync"     // @dev Math for rounding float to integer

	"github.com/nats-io/nats.go"
	// @dev Ticker for 1-second intervals
	// @dev NATS client for publishing to brla.funding
)

type Trust struct {
	ID      string
	Wallets map[string]float64 // wallet → balance
	mu      sync.RWMutex
}

// @dev BRLA funding flow from DistoDam
// @dev Source: DistoDam publishes 40% of disto
// @dev Units: amount_rt in micro-RT (1 RT = 1,000,000 micro-RT)
// @dev Appendix U: RT Transfers
type brla_fund_flow struct {
	BrlaID        string `json:"brla_id"` // @dev Target BRLA contract ID
	AmountMicroRT int64  `json:"amount_rt"`
	Source        string `json:"source"`       // @dev Wallet or node source
	TrustWallet   string `json:"trust_wallet"` // @dev Trust wallet to credit
}

var (
	trusts   = make(map[string]*Trust)
	trustsMu sync.RWMutex
)

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

	// @dev Subscribe to funding flows from DistoDam
	// @dev Topic: brla.funding
	// @dev Each message = 40% of 1-second disto from one node
	// @dev Aggregation happens here
	_, err = nc.Subscribe("brla.funding", func(m *nats.Msg) {
		var flow brla_fund_flow
		if err := json.Unmarshal(m.Data, &flow); err != nil {
			log.Printf("Invalid JSON: %v", err)
			return
		}

		// @dev Pay to Trust wallet
		payTrust(flow.TrustWallet, flow.AmountMicroRT)
	})

	// @dev Start HTTP server
	// @dev Port: 8080 (mapped to host 8081)
	// @dev Blocks until fatal error
	log.Println("Trust listening on brla.funding")

	http.HandleFunc("/pay", func(w http.ResponseWriter, r *http.Request) {
		// @dev Oracle calls /pay?trust_id=...&wallet=...&amount=...
		// @dev Add to wallet
	})

	log.Println("Trust Service running on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

func payTrust(wallet string, microRT int64) {
	trustsMu.Lock()
	defer trustsMu.Unlock()

	if _, exists := trusts[wallet]; !exists {
		trusts[wallet] = &Trust{
			ID:      wallet,
			Wallets: make(map[string]float64),
		}
	}

	amountRT := float64(microRT) / 1_000_000
	trusts[wallet].Wallets[wallet] += amountRT
	log.Printf("PAYING TRUST: %s += %.6f RT", wallet, amountRT)
}
