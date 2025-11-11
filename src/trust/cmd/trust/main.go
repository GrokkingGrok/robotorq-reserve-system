package main

import (
	// @dev Serialize flow to JSON for NATS
	"encoding/json"
	"log"      // @dev Logging to stdout (Docker logs)
	"net/http" // @dev HTTP server for /health endpoint
	"strconv"
	"sync" // @dev Math for rounding float to integer

	"github.com/nats-io/nats.go"
	// @dev Ticker for 1-second intervals
	// @dev NATS client for publishing to brla.funding
)

type Trust struct {
	ID      string  // torq1xyz...
	Balance float64 // RT
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
	if _, err := nc.Subscribe("brla.funding", func(m *nats.Msg) {
		var flow brla_fund_flow
		if err := json.Unmarshal(m.Data, &flow); err != nil {
			log.Printf("Invalid JSON: %v", err)
			return
		}
		addInflow(flow.BrlaID, "distodam", float64(flow.AmountMicroRT)/1_000_000)

	}); err != nil {
		log.Fatal("Failed to subscribe to brla.funding: ", err)
	}

	// @dev Inflow from Investor (HTTP)
	http.HandleFunc("/invest", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		addInflow(brlaID, "investor", amount)
		w.Write([]byte("Invested"))
	})

	// @dev Inflow from Builder (HTTP)
	http.HandleFunc("/retainer", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		addInflow(brlaID, "builder", amount)
		w.Write([]byte("Retainer paid"))
	})

	// @dev Inflow from Customer (HTTP)
	http.HandleFunc("/sale", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		addInflow(brlaID, "customer", amount)
		w.Write([]byte("Sale recorded"))
	})

	// @dev Pay Mint (TokenTorq)
	http.HandleFunc("/pay-mint", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		payOutflow(brlaID, "mint", amount)
		w.Write([]byte("Paid Mint"))
	})

	// @dev Pay Supplier
	http.HandleFunc("/pay-supplier", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		payOutflow(brlaID, "supplier", amount)
		w.Write([]byte("Paid Supplier"))
	})

	// @dev Pay Investor
	http.HandleFunc("/pay-investor", func(w http.ResponseWriter, r *http.Request) {
		brlaID := r.URL.Query().Get("brla_id")
		amount := parseFloat(r.URL.Query().Get("amount"))
		payOutflow(brlaID, "investor", amount)
		w.Write([]byte("Paid Investor"))
	})

	log.Println("Trust Service running on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

// @dev Parse float64 from query string
// @dev Edge: invalid → 0
func parseFloat(s string) float64 {
	f, err := strconv.ParseFloat(s, 64)
	if err != nil {
		return 0
	}
	return f
}

func addInflow(brlaID, source string, amountRT float64) {
	trustsMu.Lock()
	if _, ok := trusts[brlaID]; !ok {
		trusts[brlaID] = &Trust{}
	}
	t := trusts[brlaID]
	trustsMu.Unlock()

	t.mu.Lock()
	t.Balance += amountRT
	t.mu.Unlock()

	log.Printf("INFLOW: %s → %s: %.6f RT", source, brlaID, amountRT)
}

func payOutflow(brlaID, dest string, amountRT float64) {
	trustsMu.Lock()
	t, ok := trusts[brlaID]
	trustsMu.Unlock()
	if !ok || t.Balance < amountRT {
		log.Printf("INSUFFICIENT: BRLA %s", brlaID)
		return
	}

	t.mu.Lock()
	t.Balance -= amountRT
	t.mu.Unlock()

	log.Printf("OUTFLOW: BRLA %s → %s: %.6f RT", brlaID, dest, amountRT)
}
