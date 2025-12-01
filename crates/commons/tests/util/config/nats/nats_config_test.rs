//! Tests for NATS configuration structures.
//!
//! Exercises auth variants, JetStream defaults and bounds, and top-level defaults.
use commons::util::config::services::nats::{
    NatsAuthConfig, NatsAuthMethod, JetStreamConfig, StreamDefaultsConfig, ConsumerDefaultsConfig, NatsConfig,
};

#[test]
fn nats_auth_variants_exercised() {
    /// Covers `NatsAuthMethod` variants and associated optional fields.
    let mut auth = NatsAuthConfig::default();
    auth.method = NatsAuthMethod::None;
    assert!(matches!(auth.method, NatsAuthMethod::None));
    auth.method = NatsAuthMethod::UserPass;
    auth.username = Some("user".into());
    auth.password = Some("pass".into());
    assert!(matches!(auth.method, NatsAuthMethod::UserPass));
    auth.method = NatsAuthMethod::Token;
    auth.token = Some("t".into());
    assert!(matches!(auth.method, NatsAuthMethod::Token));
    auth.method = NatsAuthMethod::Credentials;
    auth.credentials_file = Some("/tmp/x.creds".into());
    assert!(matches!(auth.method, NatsAuthMethod::Credentials));
}

#[test]
fn jetstream_defaults_and_bounds() {
    /// Validates JetStream defaults and allows setting bounded stream/consumer values.
    let mut js = JetStreamConfig::default();
    assert!(js.enabled);
    js.stream_defaults = StreamDefaultsConfig { max_messages: 10, max_bytes: 1024, ..Default::default() };
    js.consumer_defaults = ConsumerDefaultsConfig { max_ack_pending: 10, ack_wait_seconds: 5, max_deliver: 2 };
    assert_eq!(js.stream_defaults.max_messages, 10);
    assert_eq!(js.consumer_defaults.max_deliver, 2);
}

#[test]
fn nats_config_defaults() {
    /// Confirms `NatsConfig::default()` enables NATS and sets at least one server.
    let cfg = NatsConfig::default();
    assert!(cfg.enabled);
    assert!(!cfg.servers.is_empty());
}
