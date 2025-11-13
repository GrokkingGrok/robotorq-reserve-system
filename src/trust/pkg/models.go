package pkg

// ------------------------------
// Domain Types for the Trust Node
// ------------------------------

// Trust represents a managed pool of funds for a specific project or purpose.
type Trust struct {
	ID      string  // Unique identifier for the trust
	Balance float64 // Current balance in RT (Robotorq tokens)
}

// Contract represents a "deal" that the Trust is managing.
type Contract struct {
	ID                 string  // Unique identifier for the contract
	Builder            string  // Who is building/executing the contract
	RoboStake          float64 // Amount of RoboStake (in RT) allocated
	Authorized         bool    // Has the contract been approved?
	Torq               int16   // Amount of RoboTorq being staked
	MaxTokenThroughput int64   // Max tokens allowed per interval
	IntervalSeconds    int64   // Time interval for token usage
	TotalTokens        int64   // Total tokens allocated
}

// Opportunity represents a business plan or project that might get funding.
type Opportunity struct {
	ID                 string  // Unique identifier for the opportunity
	Builder            string  // Who is building/executing the opportunity
	Description        string  // Short summary of the plan
	RoboStakeRequested float64 // How much funding (in RT) is needed
	ExpectedROI        float64 // Expected return on investment percentage
}

// FundingEvent represents money coming into a Trust.
type FundingEvent struct {
	TrustID string  // Which trust is receiving funds
	Amount  float64 // Amount of RT being added
	Source  string  // Who is sending the money
}

// OutflowEvent represents money going out of a Trust.
type OutflowEvent struct {
	TrustID string  // Which trust is sending funds
	Amount  float64 // Amount of RT being sent
	Dest    string  // Who is receiving the funds
}
