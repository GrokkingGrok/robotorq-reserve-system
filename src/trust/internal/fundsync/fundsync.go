package fundsync

import (
	"b2b/trust/pkg"
	"log"
)

// FundSync handles notifying DistoDam and tracking RoboStake for approved contracts
type FundSync struct{}

// NewFundSync returns a new FundSync instance
func NewFundSync() *FundSync {
	return &FundSync{}
}

// NotifyNewContract simulates sending contract info to DistoDam to request RoboStake
func (fs *FundSync) NotifyNewContract(op *pkg.Opportunity) *pkg.Contract {
	log.Printf("💸 FundSync: Requesting RoboStake for contract %s", op.ID)

	// Mock DistoDam response with full requested stake
	contract := &pkg.Contract{
		ID:        op.ID,
		Builder:   op.Builder,
		RoboStake: op.RoboStakeRequested,
	}
	log.Printf("✅ FundSync: RoboStake allocated %.2f for contract %s", contract.RoboStake, contract.ID)
	return contract
}
