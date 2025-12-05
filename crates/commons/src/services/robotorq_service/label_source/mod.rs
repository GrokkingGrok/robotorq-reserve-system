//! Label derivation helpers for service metrics.
//!
//! These functions extract standardized label values from `RoboTorqConfig`
//! for use in `ServiceMetricsContext`. They ensure consistent naming and
//! fallbacks for metrics labels (service, component, version, subject).

use crate::util::config::RoboTorqConfig;

/// Derived label set for metrics context.
#[derive(Debug, Clone)]
pub struct LabelSet {
    /// Service name label for metrics.
    pub service: String,
    /// Component name label for metrics.
    pub component: String,
    /// Version label for metrics.
    pub version: String,
    /// Subject label for metrics (sim/core).
    pub subject: String,
}

/// Derive service label from config.
///
/// Uses `cfg.observability.service_name`; falls back to "unknown" if empty.
///
/// # Examples
///
/// ```rust,no_run
/// use commons::util::config::RoboTorqConfig;
/// use commons::services::robotorq_service::label_source::derive_service_label;
///
/// let cfg = RoboTorqConfig {
///     schema_version: 1,
///     mode: commons::util::config::Mode::Production,
///     simulation: Default::default(),
///     ports: commons::util::config::load_ports_config_from_default(),
///     http: Default::default(),
///     nats: Default::default(),
///     persistence: Default::default(),
///     observability: Default::default(),
///     security: Default::default(),
///     crypto: Default::default(),
///     economic: Default::default(),
/// };
/// let service = derive_service_label(&cfg);
/// assert!(!service.is_empty());
/// ```
#[must_use]
pub fn derive_service_label(cfg: &RoboTorqConfig) -> String {
    let name = cfg.observability.service_name.clone();
    if name.is_empty() {
        "unknown".to_string()
    } else {
        name
    }
}

/// Derive component label from config.
///
/// Fixed to "http" for HTTP server components.
///
/// # Examples
///
/// ```
/// use commons::util::config::RoboTorqConfig;
/// use commons::services::robotorq_service::label_source::derive_component_label;
///
/// let cfg = RoboTorqConfig {
///     schema_version: 1,
///     mode: commons::util::config::Mode::Production,
///     simulation: Default::default(),
///     ports: commons::util::config::load_ports_config_from_default(),
///     http: Default::default(),
///     nats: Default::default(),
///     persistence: Default::default(),
///     observability: Default::default(),
///     security: Default::default(),
///     crypto: Default::default(),
///     economic: Default::default(),
/// };
/// assert_eq!(derive_component_label(&cfg), "http");
/// ```
#[must_use]
pub fn derive_component_label(_cfg: &RoboTorqConfig) -> String {
    "http".to_string()
}

/// Derive version label from config.
///
/// Uses `cfg.observability.service_version`.
///
/// # Examples
///
/// ```
/// use commons::util::config::RoboTorqConfig;
/// use commons::services::robotorq_service::label_source::derive_version_label;
///
/// let cfg = RoboTorqConfig {
///     schema_version: 1,
///     mode: commons::util::config::Mode::Production,
///     simulation: Default::default(),
///     ports: commons::util::config::load_ports_config_from_default(),
///     http: Default::default(),
///     nats: Default::default(),
///     persistence: Default::default(),
///     observability: Default::default(),
///     security: Default::default(),
///     crypto: Default::default(),
///     economic: Default::default(),
/// };
/// let version = derive_version_label(&cfg);
/// assert!(!version.is_empty());
/// ```
#[must_use]
pub fn derive_version_label(cfg: &RoboTorqConfig) -> String {
    cfg.observability.service_version.clone()
}

/// Derive subject label from config.
///
/// "sim" if simulation mode; else "core".
///
/// # Examples
///
/// ```
/// use commons::util::config::RoboTorqConfig;
/// use commons::services::robotorq_service::label_source::derive_subject_label;
///
/// let cfg = RoboTorqConfig {
///     schema_version: 1,
///     mode: commons::util::config::Mode::Production,
///     simulation: Default::default(),
///     ports: commons::util::config::load_ports_config_from_default(),
///     http: Default::default(),
///     nats: Default::default(),
///     persistence: Default::default(),
///     observability: Default::default(),
///     security: Default::default(),
///     crypto: Default::default(),
///     economic: Default::default(),
/// };
/// let subject = derive_subject_label(&cfg);
/// assert!(!subject.is_empty());
/// ```
#[must_use]
pub fn derive_subject_label(cfg: &RoboTorqConfig) -> String {
    match cfg.mode {
        crate::util::config::Mode::Simulation => "sim".to_string(),
        crate::util::config::Mode::Production => "core".to_string(),
    }
}

/// Build full label set from config.
///
/// # Examples
///
/// ```
/// use commons::util::config::RoboTorqConfig;
/// use commons::services::robotorq_service::label_source::{build_label_set, LabelSet};
///
/// let cfg = RoboTorqConfig {
///     schema_version: 1,
///     mode: commons::util::config::Mode::Production,
///     simulation: Default::default(),
///     ports: commons::util::config::load_ports_config_from_default(),
///     http: Default::default(),
///     nats: Default::default(),
///     persistence: Default::default(),
///     observability: Default::default(),
///     security: Default::default(),
///     crypto: Default::default(),
///     economic: Default::default(),
/// };
/// let labels = build_label_set(&cfg);
/// assert!(matches!(labels, LabelSet { .. }));
/// ```
#[must_use]
pub fn build_label_set(cfg: &RoboTorqConfig) -> LabelSet {
    LabelSet {
        service: derive_service_label(cfg),
        component: derive_component_label(cfg),
        version: derive_version_label(cfg),
        subject: derive_subject_label(cfg),
    }
}

/// Abstraction for deriving metric labels without binding to `RoboTorqConfig`.
pub trait MetricsLabelProvider: Send + Sync {
    /// Produce a complete `LabelSet` for metrics.
    fn labels(&self) -> LabelSet;
}

impl MetricsLabelProvider for RoboTorqConfig {
    fn labels(&self) -> LabelSet {
        build_label_set(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_service_label_fallback() {
        let cfg = crate::util::config::RoboTorqConfig {
            schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
            mode: crate::util::config::Mode::Production,
            simulation: Default::default(),
            ports: crate::util::config::load_ports_config_from_default(),
            http: Default::default(),
            nats: Default::default(),
            persistence: Default::default(),
            observability: crate::util::config::ObservabilityConfig {
                service_name: String::new(), // Empty to test fallback
                service_instance: Default::default(),
                service_version: Default::default(),
                metrics: Default::default(),
                tracing: Default::default(),
                logging: Default::default(),
                health: Default::default(),
            },
            security: Default::default(),
            crypto: Default::default(),
            economic: Default::default(),
        };
        assert_eq!(derive_service_label(&cfg), "unknown");
    }

    #[test]
    fn test_derive_component_label_fixed() {
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        assert_eq!(derive_component_label(&cfg), "http");
    }

    #[test]
    fn test_derive_version_label_fallback() {
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        let version = derive_version_label(&cfg);
        assert!(!version.is_empty());
    }

    #[test]
    fn test_derive_subject_label_simulation() {
        let cfg = crate::util::config::RoboTorqConfig {
            schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
            mode: crate::util::config::Mode::Simulation,
            simulation: Default::default(),
            ports: crate::util::config::load_ports_config_from_default(),
            http: Default::default(),
            nats: Default::default(),
            persistence: Default::default(),
            observability: Default::default(),
            security: Default::default(),
            crypto: Default::default(),
            economic: Default::default(),
        };
        assert_eq!(derive_subject_label(&cfg), "sim");
    }

    #[test]
    fn test_derive_subject_label_core() {
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        assert_eq!(derive_subject_label(&cfg), "core");
    }

    #[test]
    fn test_build_label_set() {
        let cfg = crate::util::config::load_robotorq_config(None).unwrap_or_else(|_| {
            crate::util::config::RoboTorqConfig {
                schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
                mode: crate::util::config::Mode::Production,
                simulation: Default::default(),
                ports: crate::util::config::load_ports_config_from_default(),
                http: Default::default(),
                nats: Default::default(),
                persistence: Default::default(),
                observability: Default::default(),
                security: Default::default(),
                crypto: Default::default(),
                economic: Default::default(),
            }
        });
        let labels = build_label_set(&cfg);
        assert_eq!(labels.component, "http");
        assert_eq!(labels.subject, "core");
    }

    #[test]
    fn test_derive_service_label_explicit() {
        let obs = crate::util::config::ObservabilityConfig {
            service_name: "my-service".to_string(),
            ..Default::default()
        };
        let cfg = crate::util::config::RoboTorqConfig {
            schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
            mode: crate::util::config::Mode::Production,
            simulation: Default::default(),
            ports: crate::util::config::load_ports_config_from_default(),
            http: Default::default(),
            nats: Default::default(),
            persistence: Default::default(),
            observability: obs,
            security: Default::default(),
            crypto: Default::default(),
            economic: Default::default(),
        };
        assert_eq!(derive_service_label(&cfg), "my-service");
    }

    #[test]
    fn test_metrics_label_provider_custom_impl() {
        struct CustomProvider;
        impl MetricsLabelProvider for CustomProvider {
            fn labels(&self) -> LabelSet {
                LabelSet {
                    service: "custom".to_string(),
                    component: "worker".to_string(),
                    version: "vX".to_string(),
                    subject: "core".to_string(),
                }
            }
        }
        let p = CustomProvider;
        let labels = p.labels();
        assert_eq!(labels.service, "custom");
        assert_eq!(labels.component, "worker");
        assert_eq!(labels.version, "vX");
    }

    #[test]
    fn test_derive_service_label_nonempty() {
        let obs = crate::util::config::ObservabilityConfig {
            service_name: "my-service".to_string(),
            ..Default::default()
        };
        let cfg = crate::util::config::RoboTorqConfig {
            schema_version: crate::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION,
            mode: crate::util::config::Mode::Production,
            simulation: Default::default(),
            ports: crate::util::config::load_ports_config_from_default(),
            http: Default::default(),
            nats: Default::default(),
            persistence: Default::default(),
            observability: obs,
            security: Default::default(),
            crypto: Default::default(),
            economic: Default::default(),
        };
        assert_eq!(derive_service_label(&cfg), "my-service");
    }

    struct StaticProvider(LabelSet);
    impl MetricsLabelProvider for StaticProvider {
        fn labels(&self) -> LabelSet {
            self.0.clone()
        }
    }

    #[test]
    fn test_custom_metrics_label_provider() {
        let provider = StaticProvider(LabelSet {
            service: "custom".to_string(),
            component: "http".to_string(),
            version: "v1".to_string(),
            subject: "core".to_string(),
        });
        let labels = provider.labels();
        assert_eq!(labels.service, "custom");
        assert_eq!(labels.version, "v1");
    }
}
