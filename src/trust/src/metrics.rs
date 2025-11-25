use once_cell::sync::Lazy;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder};

pub struct Metrics {
    pub registry: Registry,
    pub requests_total: IntCounter,
    pub requests_bad: IntCounter,
    pub requests_not_found: IntCounter,
    pub request_latency: Histogram,
    pub served_contracts: IntGauge,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounter::with_opts(Opts::new("trust_requests_total", "Total contract requests received")).unwrap();
        let requests_bad = IntCounter::with_opts(Opts::new("trust_requests_bad_total", "Bad requests received")).unwrap();
        let requests_not_found = IntCounter::with_opts(Opts::new("trust_requests_not_found_total", "Requests for missing contracts")).unwrap();
        let served_contracts = IntGauge::with_opts(Opts::new("trust_served_contracts", "Number of served contracts in memory")).unwrap();
        let request_latency = Histogram::with_opts(HistogramOpts::new("trust_request_latency_seconds", "Request handling latency in seconds")).unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(requests_bad.clone())).unwrap();
        registry.register(Box::new(requests_not_found.clone())).unwrap();
        registry.register(Box::new(served_contracts.clone())).unwrap();
        registry.register(Box::new(request_latency.clone())).unwrap();

        Metrics { registry, requests_total, requests_bad, requests_not_found, request_latency, served_contracts }
    }

    pub fn gather_prometheus(&self) -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let mfs = self.registry.gather();
        encoder.encode(&mfs, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

pub static METRICS: Lazy<Metrics> = Lazy::new(|| Metrics::new());
