// Package mint provides Prometheus metrics for Phase3DistoDamPublisher.
package mint

import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// Phase3DistoDamPublisherMetrics holds Prometheus metrics for Phase3 publisher.
type Phase3DistoDamPublisherMetrics struct {
	// UnitsPublishedTotal counts successful Phase3RoboTorqUnit publishes
	UnitsPublishedTotal prometheus.Counter

	// PublishErrorsTotal counts failed publishes (after all retries)
	PublishErrorsTotal prometheus.Counter

	// PublishLatency measures time to publish (including retries)
	PublishLatency prometheus.Histogram

	// RetryAttemptsTotal counts total retry attempts
	RetryAttemptsTotal prometheus.Counter
}

// NewPhase3DistoDamPublisherMetrics creates Prometheus metrics for Phase3 publisher.
//
// Parameters:
//   - registry: Prometheus registry (uses default if nil)
//
// Returns initialized metrics struct.
func NewPhase3DistoDamPublisherMetrics(registry prometheus.Registerer) *Phase3DistoDamPublisherMetrics {
	if registry == nil {
		// Use promauto for default registry
		return &Phase3DistoDamPublisherMetrics{
			UnitsPublishedTotal: promauto.NewCounter(prometheus.CounterOpts{
				Name: "mint_phase3_units_published_total",
				Help: "Total number of Phase3RoboTorqUnit successfully published to NATS",
			}),

			PublishErrorsTotal: promauto.NewCounter(prometheus.CounterOpts{
				Name: "mint_phase3_publish_errors_total",
				Help: "Total number of Phase3RoboTorqUnit publish failures (after retries)",
			}),

			PublishLatency: promauto.NewHistogram(prometheus.HistogramOpts{
				Name:    "mint_phase3_publish_seconds",
				Help:    "Time spent publishing Phase3RoboTorqUnit to NATS (including retries)",
				Buckets: prometheus.DefBuckets,
			}),

			RetryAttemptsTotal: promauto.NewCounter(prometheus.CounterOpts{
				Name: "mint_phase3_retry_attempts_total",
				Help: "Total number of Phase3RoboTorqUnit publish retry attempts",
			}),
		}
	}

	// Use custom registry
	return &Phase3DistoDamPublisherMetrics{
		UnitsPublishedTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase3_units_published_total",
			Help: "Total number of Phase3RoboTorqUnit successfully published to NATS",
		}),

		PublishErrorsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase3_publish_errors_total",
			Help: "Total number of Phase3RoboTorqUnit publish failures (after retries)",
		}),

		PublishLatency: prometheus.NewHistogram(prometheus.HistogramOpts{
			Name:    "mint_phase3_publish_seconds",
			Help:    "Time spent publishing Phase3RoboTorqUnit to NATS (including retries)",
			Buckets: prometheus.DefBuckets,
		}),

		RetryAttemptsTotal: prometheus.NewCounter(prometheus.CounterOpts{
			Name: "mint_phase3_retry_attempts_total",
			Help: "Total number of Phase3RoboTorqUnit publish retry attempts",
		}),
	}
}
