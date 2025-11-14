package appraiser

type Contract struct {
	ID                 string
	OpportunityID      string
	Builder            string
	RoboStake          int
	Torq               int
	MaxTokenThroughput int
	IntervalSeconds    int
	TotalTokens        int
}
