// Prometheus metrics for Digger
//
// Tracks:
// - Contract lifecycle (create/stake/execute)
// - Printer interactions (job start/milestone/complete)
// - JTU generation and storage
// - Hash batch transmission

use prometheus::{
    Counter, CounterVec, Histogram, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;

/// Digger metrics collector
#[derive(Clone)]
pub struct DiggerMetrics {
    // Contract metrics
    pub contracts_created_total: IntCounter,
    pub contracts_staked_total: IntCounter,
    pub contracts_executed_total: IntCounter,
    pub contracts_active: IntGauge,
    
    // Printer interaction metrics
    pub printer_jobs_started_total: IntCounter,
    pub printer_milestones_reported_total: IntCounter,
    pub printer_jobs_completed_total: IntCounter,
    
    // JTU metrics
    pub jtus_generated_total: IntCounter,
    pub jtus_stored_total: IntCounter,
    pub ore_generated_total: Counter,
    
    // Hash transmission metrics
    pub hash_batches_sent_total: IntCounter,
    pub hashes_sent_total: IntCounter,
    pub robostake_sent_total: Counter,
    
    // API latency metrics
    pub api_request_duration: HistogramVec,
    
    // Error metrics
    pub api_errors_total: IntCounterVec,
    
    // Registry (for /metrics endpoint)
    registry: Arc<Registry>,
}

impl DiggerMetrics {
    /// Create new metrics collector with default registry
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Contract metrics
        let contracts_created_total = IntCounter::with_opts(
            Opts::new("digger_contracts_created_total", "Total contracts created")
        )?;
        registry.register(Box::new(contracts_created_total.clone()))?;
        
        let contracts_staked_total = IntCounter::with_opts(
            Opts::new("digger_contracts_staked_total", "Total contracts with stake paid")
        )?;
        registry.register(Box::new(contracts_staked_total.clone()))?;
        
        let contracts_executed_total = IntCounter::with_opts(
            Opts::new("digger_contracts_executed_total", "Total contracts executed")
        )?;
        registry.register(Box::new(contracts_executed_total.clone()))?;
        
        let contracts_active = IntGauge::with_opts(
            Opts::new("digger_contracts_active", "Number of active contracts")
        )?;
        registry.register(Box::new(contracts_active.clone()))?;
        
        // Printer interaction metrics
        let printer_jobs_started_total = IntCounter::with_opts(
            Opts::new("digger_printer_jobs_started_total", "Total printer jobs started")
        )?;
        registry.register(Box::new(printer_jobs_started_total.clone()))?;
        
        let printer_milestones_reported_total = IntCounter::with_opts(
            Opts::new("digger_printer_milestones_reported_total", "Total milestones reported by printers")
        )?;
        registry.register(Box::new(printer_milestones_reported_total.clone()))?;
        
        let printer_jobs_completed_total = IntCounter::with_opts(
            Opts::new("digger_printer_jobs_completed_total", "Total printer jobs completed")
        )?;
        registry.register(Box::new(printer_jobs_completed_total.clone()))?;
        
        // JTU metrics
        let jtus_generated_total = IntCounter::with_opts(
            Opts::new("digger_jtus_generated_total", "Total JTUs generated")
        )?;
        registry.register(Box::new(jtus_generated_total.clone()))?;
        
        let jtus_stored_total = IntCounter::with_opts(
            Opts::new("digger_jtus_stored_total", "Total JTUs stored in database")
        )?;
        registry.register(Box::new(jtus_stored_total.clone()))?;
        
        let ore_generated_total = Counter::with_opts(
            Opts::new("digger_ore_generated_total", "Total RT ore generated")
        )?;
        registry.register(Box::new(ore_generated_total.clone()))?;
        
        // Hash transmission metrics
        let hash_batches_sent_total = IntCounter::with_opts(
            Opts::new("digger_hash_batches_sent_total", "Total hash batches sent to NATS")
        )?;
        registry.register(Box::new(hash_batches_sent_total.clone()))?;
        
        let hashes_sent_total = IntCounter::with_opts(
            Opts::new("digger_hashes_sent_total", "Total hashes sent to NATS")
        )?;
        registry.register(Box::new(hashes_sent_total.clone()))?;
        
        let robostake_sent_total = Counter::with_opts(
            Opts::new("digger_robostake_sent_total", "Total RoboStake sent in ore batches")
        )?;
        registry.register(Box::new(robostake_sent_total.clone()))?;
        
        // API latency metrics
        let api_request_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "digger_api_request_duration_seconds",
                "API request duration in seconds"
            ),
            &["endpoint", "method"]
        )?;
        registry.register(Box::new(api_request_duration.clone()))?;
        
        // Error metrics
        let api_errors_total = IntCounterVec::new(
            Opts::new("digger_api_errors_total", "Total API errors"),
            &["endpoint", "error_type"]
        )?;
        registry.register(Box::new(api_errors_total.clone()))?;
        
        Ok(Self {
            contracts_created_total,
            contracts_staked_total,
            contracts_executed_total,
            contracts_active,
            printer_jobs_started_total,
            printer_milestones_reported_total,
            printer_jobs_completed_total,
            jtus_generated_total,
            jtus_stored_total,
            ore_generated_total,
            hash_batches_sent_total,
            hashes_sent_total,
            robostake_sent_total,
            api_request_duration,
            api_errors_total,
            registry: Arc::new(registry),
        })
    }
    
    /// Get the registry for /metrics endpoint
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    
    /// Gather all metrics for Prometheus scraping
    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
}

impl Default for DiggerMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}
