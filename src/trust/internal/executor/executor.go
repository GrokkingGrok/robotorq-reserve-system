package executor

import (
	"b2b/trust/pkg"
	"log"
	"time"
)

// Executor ensures the contract is executed correctly
type Executor struct{}

// NewExecutor returns a new Executor instance
func NewExecutor() *Executor {
	return &Executor{}
}

// SetupContract mocks pinging the Builder/Digger node
func (e *Executor) SetupContract(c *pkg.Contract) {
	log.Printf("⚙️ Executor: Setting up contract %s with Builder %s", c.ID, c.Builder)

	// Mock Digger startup
	log.Printf("🔄 Executor: Pinging Builder's Digger node for readiness...")
	time.Sleep(1 * time.Second) // simulate network/boot time

	log.Printf("✅ Executor: Builder's Digger is ready for contract %s", c.ID)

	// Mock sending RoboStake
	e.SendRoboStake(c)
}

// SendRoboStake sends the RoboStake to the Builder
func (e *Executor) SendRoboStake(c *pkg.Contract) {
	log.Printf("💰 Executor: Sending RoboStake %.2f to Builder %s for contract %s",
		c.RoboStake, c.Builder, c.ID)
}
