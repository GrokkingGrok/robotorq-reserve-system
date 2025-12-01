use commons::util::config::observability::{TracingBackend, TracingConfig, MetricsRegistryType, MetricsConfig, ObservabilityConfig};

#[test]
fn tracing_config_variants_default_ok() {
    let mut cfg = TracingConfig::default();
    cfg.backend = TracingBackend::Console;
    assert!(matches!(cfg.backend, TracingBackend::Console));

    cfg.backend = TracingBackend::Jaeger;
    assert!(matches!(cfg.backend, TracingBackend::Jaeger));

    cfg.backend = TracingBackend::OtelOtlp;
    assert!(matches!(cfg.backend, TracingBackend::OtelOtlp));
}

#[test]
fn metrics_registry_variants_ok() {
    let mut m = MetricsConfig::default();
    m.registry = MetricsRegistryType::Prometheus;
    assert!(matches!(m.registry, MetricsRegistryType::Prometheus));
}

#[test]
fn observability_toggle_defaults() {
    let o = ObservabilityConfig::default();
    assert!(!o.service_name.is_empty());
    assert!(!o.service_instance.is_empty());
    assert!(!o.service_version.is_empty());
}
