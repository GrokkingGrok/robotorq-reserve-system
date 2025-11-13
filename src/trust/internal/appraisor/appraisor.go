package appraisor

import (
	"b2b/trust/pkg"
	"log"
)

// Appraisor evaluates incoming opportunities and chooses the best one.
// Currently, this is a dumb selection (highest ROI).
type Appraisor struct{}

// New returns a new Appraisor instance
func New() *Appraisor {
	return &Appraisor{}
}

// SelectBestOpportunity picks the opportunity with the highest ROI
func (a *Appraisor) SelectBestOpportunity(ops []pkg.Opportunity) *pkg.Opportunity {
	if len(ops) == 0 {
		log.Println("⚠️ No opportunities to appraise")
		return nil
	}

	best := &ops[0]
	for i := 1; i < len(ops); i++ {
		if ops[i].ExpectedROI > best.ExpectedROI {
			best = &ops[i]
		}
	}
	log.Printf("✅ Appraisor selected opportunity %s with ROI %.2f", best.ID, best.ExpectedROI)
	return best
}

// MockBidNet simulates bidding on the selected opportunity
func (a *Appraisor) MockBidNet(op *pkg.Opportunity) {
	log.Printf("📝 Mock BidNet: Auto-bid accepted for opportunity %s", op.ID)
}
