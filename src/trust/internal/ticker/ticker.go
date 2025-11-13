// Package ticker provides utilities for periodic tasks or
// "ticks" that the Trust service might perform. For example,
// checking funding status, updating metrics, or re-evaluating bids.
package ticker

import (
	"log"
	"time"
)

// StartTicker runs a function periodically at the specified interval.
func StartTicker(interval time.Duration, job func()) {
	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()
		for range ticker.C {
			log.Println("Running scheduled job...")
			job()
		}
	}()
}
