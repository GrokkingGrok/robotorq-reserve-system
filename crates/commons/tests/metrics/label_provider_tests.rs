//! Tests for MetricsLabelProvider abstraction.
use std::sync::Arc;
use commons::services::robotorq_service::{MetricsLabelProvider, LabelSet, build_label_set};
use commons::util::config::load_robotorq_config;

struct StaticProvider;
impl MetricsLabelProvider for StaticProvider {
    fn labels(&self) -> LabelSet {
        LabelSet { service: "static_svc".into(), component: "custom_comp".into(), version: "vTEST".into(), subject: "core".into() }
    }
}

#[test]
fn config_based_provider_matches_build_label_set() {
    let cfg = load_robotorq_config(None).unwrap();
    let labels = build_label_set(&cfg);
    assert!(!labels.service.is_empty());
    assert_eq!(labels.component, "http");
}

#[test]
fn static_provider_overrides_labels() {
    let p: Arc<dyn MetricsLabelProvider> = Arc::new(StaticProvider);
    let labels = p.labels();
    assert_eq!(labels.service, "static_svc");
    assert_eq!(labels.component, "custom_comp");
    assert_eq!(labels.version, "vTEST");
    assert_eq!(labels.subject, "core");
}
