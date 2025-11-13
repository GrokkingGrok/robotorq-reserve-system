package metrics

import (
	"github.com/prometheus/client_golang/prometheus"
)

// Metrics holds Prometheus counters and gauges
type MetricRegistry struct {
	InflowCounter  *prometheus.CounterVec
	OutflowCounter *prometheus.CounterVec
	BalanceGauge   *prometheus.GaugeVec
}

// NewMetrics initializes all Prometheus metrics
func NewMetricRegistry() *MetricRegistry {
	m := &MetricRegistry{
		InflowCounter: prometheus.NewCounterVec(
			prometheus.CounterOpts{Name: "trust_inflows_total", Help: "Number of inflow operations by source"},
			[]string{"source"},
		),
		OutflowCounter: prometheus.NewCounterVec(
			prometheus.CounterOpts{Name: "trust_outflows_total", Help: "Number of outflow operations by destination"},
			[]string{"dest"},
		),
		BalanceGauge: prometheus.NewGaugeVec(
			prometheus.GaugeOpts{Name: "trust_balance_rt", Help: "Current RT balance per trust"},
			[]string{"trust_id"},
		),
	}
	prometheus.MustRegister(m.InflowCounter, m.OutflowCounter, m.BalanceGauge)
	return m
}
