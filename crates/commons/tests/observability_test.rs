use commons::util::config::observability::{
    MetricsConfig, MetricsRegistryType, ObservabilityConfig, TracingBackend, TracingConfig,
};

#[test]
fn tracing_config_variants_default_ok() {
    let cfg = TracingConfig {
        backend: TracingBackend::Console,
        ..Default::default()
    };
    assert!(matches!(cfg.backend, TracingBackend::Console));

    let cfg = TracingConfig {
        backend: TracingBackend::Jaeger,
        ..Default::default()
    };
    assert!(matches!(cfg.backend, TracingBackend::Jaeger));

    let cfg = TracingConfig {
        backend: TracingBackend::OtelOtlp,
        ..Default::default()
    };
    assert!(matches!(cfg.backend, TracingBackend::OtelOtlp));
}

#[test]
fn metrics_registry_variants_ok() {
    let m = MetricsConfig {
        registry: MetricsRegistryType::Prometheus,
        ..Default::default()
    };
    assert!(matches!(m.registry, MetricsRegistryType::Prometheus));
}

#[test]
fn observability_toggle_defaults() {
    let o = ObservabilityConfig::default();
    assert!(!o.service_name.is_empty());
    assert!(!o.service_instance.is_empty());
    assert!(!o.service_version.is_empty());
}
