package main

import (
	"log"
	"time"

	"b2b/trust/internal/appraisor"
	"b2b/trust/internal/executor"
	"b2b/trust/internal/fundsync"
	"b2b/trust/internal/metrics"
	"b2b/trust/pkg"

	"github.com/nats-io/nats.go"
)

func main() {
	// -----------------------
	// STEP 1: Initialize Metrics
	// -----------------------
	m := metrics.NewMetricRegistry()
	log.Println("Prometheus metrics initialized")

	// -----------------------
	// STEP 2: Connect to NATS
	// -----------------------
	nc, err := nats.Connect(nats.DefaultURL)
	if err != nil {
		log.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()
	log.Println("Connected to NATS")

	// Example subscription: listen for new opportunities (mocked)
	if _, err := nc.Subscribe("bidnet.opportunities", func(msg *nats.Msg) {
		log.Printf("Received message on bidnet.opportunities: %s", string(msg.Data))
		// In a real system, decode msg.Data into pkg.Opportunity
	}); err != nil {
		log.Fatalf("Failed to subscribe to bidnet.opportunities: %v", err)
	}
	if err := nc.Flush(); err != nil {
		log.Fatalf("Failed to flush NATS connection: %v", err)
	}

	// -----------------------
	// STEP 3: Instantiate Trust Node Components
	// -----------------------
	trust := pkg.Trust{
		ID:      "trust-001",
		Balance: 1000.0, // Starting with 1000 RT
	}

	app := appraisor.New()
	fs := fundsync.NewFundSync()
	exec := executor.NewExecutor()

	// -----------------------
	// STEP 4: Simulate Business Loop
	// -----------------------
	for i := 1; i <= 3; i++ { // Loop 3 times for demo
		log.Printf("🔄 Starting business cycle %d", i)

		// Step 4a: Create a fake opportunity (simulates BidNet message)
		op := pkg.Opportunity{
			ID:                 "opp-001",
			Builder:            "builder-123",
			RoboStakeRequested: 500.0,
			ExpectedROI:        1.2,
		}
		log.Printf("📥 New opportunity received: %s", op.ID)

		// Step 4b: Appraisor selects the best opportunity
		best := app.SelectBestOpportunity([]pkg.Opportunity{op})
		if best == nil {
			log.Println("No suitable opportunity selected, skipping cycle")
			continue
		}

		// Step 4c: Mock BidNet auto-accept
		app.MockBidNet(best)

		// Step 4d: FundSync requests RoboStake from DistoDam (mocked)
		contract := fs.NotifyNewContract(best)

		// Step 4e: Executor sets up contract and sends RoboStake
		exec.SetupContract(contract)

		// Step 4f: Update metrics (simulated)
		m.BalanceGauge.WithLabelValues(trust.ID).Set(trust.Balance)
		m.InflowCounter.WithLabelValues("demo").Inc()

		log.Printf("✅ Business cycle %d complete\n", i)
		time.Sleep(1 * time.Second) // pause between cycles
	}

	log.Println("🛑 Demo loop complete, Trust service exiting")
}
