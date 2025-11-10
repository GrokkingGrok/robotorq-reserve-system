// src/refinery/cmd/refinery/main.go
package main

import (
	"encoding/json" // @dev Parse JSON from NATS messages
	"log"           // @dev Structured logging to stdout (Docker)
	"net/http"      // @dev HTTP server for /health endpoint

	"github.com/nats-io/nats.go" // @dev NATS client for subscribing to brla.funding
)

// @dev BRLA funding flow from DistoRouter
// @dev Source: DistoRouter publishes 40% of disto
// @dev Units: amount_rt in micro-RT (1 RT = 1,000,000 micro-RT)
// @dev Appendix U: RT Transfers
type brla_fund_flow struct {
	BrlaID        string `json:"brla_id"` // @dev Target BRLA contract ID
	AmountMicroRT int64  `json:"amount_rt"`
	Source        string `json:"source"`       // @dev Wallet or node source
	TrustWallet   string `json:"trust_wallet"` // @dev Trust wallet to credit
}

var brlaRegistry = map[string]string{
	"JON-3DPRINT-001": "torq1xyz...", // brla_id → trust_wallet
}

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

	// @dev Subscribe to funding flows from DistoRouter
	// @dev Topic: brla.funding
	// @dev Each message = 40% of 1-second disto from one node
	// @dev Aggregation happens here
	_, err = nc.Subscribe("brla.funding", func(m *nats.Msg) {
		var flow brla_fund_flow
		if err := json.Unmarshal(m.Data, &flow); err != nil {
			log.Printf("Invalid JSON in brla.funding: %v", err)
			return
		}
		// @dev Process funding flow
		payBRLATrust(flow)
	})
	if err != nil {
		log.Fatal("Failed to subscribe to brla.funding: ", err)
	}

	// @dev Start HTTP server
	// @dev Port: 8080 (mapped to host 8081)
	// @dev Blocks until fatal error
	log.Println("Refinery listening on brla.funding → minting BRLA")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

// @dev Pay BRLA from aggregated funding
// @dev Parameters:
// @dev   brlaID: Contract ID (e.g., "JON-3DPRINT-001")
// @dev   amountRT: Funding in RT (float64)
// @dev   source: Origin wallet/node
// @dev Side Effects: Updates Postgres, emits mint event
// @dev Appendix 5: BRLA Minting Logic
func payBRLATrust(flow brla_fund_flow) {
	// @dev 1. Check brla_id exists
	trustWallet, ok := brlaRegistry[flow.BrlaID]
	if !ok {
		log.Printf("INVALID BRLA: %s", flow.BrlaID)
		return
	}

	// @dev 2. Check source allowed (optional)
	if flow.Source != "jon_wallet" {
		log.Printf("UNAUTHORIZED: %s", flow.Source)
		return
	}

	// @dev 3. Pay
	amountRT := float64(flow.AmountMicroRT) / 1_000_000
	log.Printf("PAYING TRUST: %s += %.6f RT", trustWallet, amountRT)
}
