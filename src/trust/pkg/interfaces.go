package pkg

// ------------------------
// Interfaces for the Trust
// ------------------------

// TrustOperator defines what a Trust should be able to do.
type TrustOperator interface {
	AddInflow(event FundingEvent)  // Add funds to a trust
	PayOutflow(event OutflowEvent) // Remove funds from a trust
	GetBalance(trustID string) float64
}

// ContractExecutor defines behavior for executing contracts.
type ContractExecutor interface {
	Execute(contract Contract) error // Run the contract
	Settle(contractID string) error  // Handle settlement if needed
}

// Appraisor defines behavior for assessing opportunities.
type Appraisor interface {
	Assess(op Opportunity) bool // Return true if Trust wants to bid
}
