use lazy_static::lazy_static;
use prometheus::{Counter, Gauge, IntCounter, IntGauge, Opts, Registry};
use std::net::SocketAddr;
use warp::Filter;

pub struct Metrics {
    pub heartbeats_sent_total: IntCounter,
    pub prints_completed_total: IntCounter,
    pub capacity_kwh_total: Counter,
    pub uptime_seconds: IntGauge,
    pub certificate_valid: Gauge,
}

lazy_static! {
    pub static ref METRICS: Metrics = Metrics::new();
    static ref REGISTRY: Registry = Registry::new();
}

impl Metrics {
    fn new() -> Self {
        let heartbeats_sent_total = IntCounter::with_opts(
            Opts::new("printer_heartbeats_sent_total", "Total heartbeats sent to Digger")
        ).unwrap();
        REGISTRY.register(Box::new(heartbeats_sent_total.clone())).unwrap();
        
        let prints_completed_total = IntCounter::with_opts(
            Opts::new("printer_prints_completed_total", "Total prints completed")
        ).unwrap();
        REGISTRY.register(Box::new(prints_completed_total.clone())).unwrap();
        
        let capacity_kwh_total = Counter::with_opts(
            Opts::new("printer_capacity_kwh_total", "Total capacity used in kWh")
        ).unwrap();
        REGISTRY.register(Box::new(capacity_kwh_total.clone())).unwrap();
        
        let uptime_seconds = IntGauge::with_opts(
            Opts::new("printer_uptime_seconds", "Printer service uptime in seconds")
        ).unwrap();
        REGISTRY.register(Box::new(uptime_seconds.clone())).unwrap();
        
        let certificate_valid = Gauge::with_opts(
            Opts::new("printer_certificate_valid", "Whether printer has valid certificate (0 or 1)")
        ).unwrap();
        REGISTRY.register(Box::new(certificate_valid.clone())).unwrap();
        
        Self {
            heartbeats_sent_total,
            prints_completed_total,
            capacity_kwh_total,
            uptime_seconds,
            certificate_valid,
        }
    }
}

pub async fn start_metrics_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let metrics_route = warp::path("metrics")
        .map(|| {
            use prometheus::Encoder;
            let encoder = prometheus::TextEncoder::new();
            let metric_families = REGISTRY.gather();
            let mut buffer = vec![];
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });
    
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    tracing::info!("📊 Metrics server listening on http://{}/metrics", addr);
    
    warp::serve(metrics_route)
        .run(addr)
        .await;
    
    Ok(())
}
