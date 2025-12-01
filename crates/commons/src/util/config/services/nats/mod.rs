//! NATS message bus configuration for RoboTorq services.
//!
//! This module defines configuration options for NATS connectivity including
//! server connections, JetStream settings, and subject naming conventions.

use serde::{Deserialize, Serialize};

/// NATS message bus configuration.
///
/// Configures connection to the NATS server for inter-service communication.
/// NATS serves as the primary messaging backbone for the distributed RoboTorq system.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::{NatsConfig, NatsAuthConfig, NatsAuthMethod};
///
/// // Development single-server configuration
/// let dev_config = NatsConfig {
///     enabled: true,
///     servers: vec!["nats://localhost:4222".to_string()],
///     connect_timeout_seconds: 5,
///     max_reconnects: 10,
///     reconnect_delay_ms: 500,
///     auth: NatsAuthConfig {
///         method: NatsAuthMethod::None,
///         ..Default::default()
///     },
///     ..Default::default()
/// };
///
/// // Production clustered configuration with authentication
/// let prod_config = NatsConfig {
///     enabled: true,
///     servers: vec![
///         "nats://nats-1.robotorq.internal:4222".to_string(),
///         "nats://nats-2.robotorq.internal:4222".to_string(),
///         "nats://nats-3.robotorq.internal:4222".to_string(),
///     ],
///     connect_timeout_seconds: 30,
///     max_reconnects: 0, // unlimited retries
///     reconnect_delay_ms: 1000,
///     auth: NatsAuthConfig {
///         method: NatsAuthMethod::Credentials,
///         credentials_file: Some("/etc/robotorq/nats.creds".to_string()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// Whether NATS connectivity is enabled.
    ///
    /// When disabled, services will not attempt to connect to NATS.
    /// Useful for testing or single-service deployments.
    #[serde(default = "default_nats_enabled")]
    pub enabled: bool,

    /// List of NATS server URLs.
    ///
    /// Multiple servers can be specified for clustering and failover.
    /// Format: "nats://host:port" or "tls://host:port" for TLS connections.
    #[serde(default = "default_nats_servers")]
    pub servers: Vec<String>,

    /// Connection timeout in seconds.
    ///
    /// Maximum time to wait for initial NATS connection establishment.
    #[serde(default = "default_nats_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,

    /// Maximum reconnection attempts.
    ///
    /// Number of times to attempt reconnection before giving up.
    /// Set to 0 for unlimited retries.
    #[serde(default = "default_nats_max_reconnects")]
    pub max_reconnects: usize,

    /// Reconnection delay in milliseconds.
    ///
    /// Base delay between reconnection attempts. May be increased exponentially.
    #[serde(default = "default_nats_reconnect_delay_ms")]
    pub reconnect_delay_ms: u64,

    /// Authentication credentials.
    ///
    /// Optional authentication configuration for NATS connections.
    #[serde(default)]
    pub auth: NatsAuthConfig,

    /// JetStream configuration.
    ///
    /// Settings for NATS JetStream durable messaging and streams.
    #[serde(default)]
    pub jetstream: JetStreamConfig,

    /// Subject naming configuration.
    ///
    /// Defines the subject hierarchy and naming conventions for messages.
    #[serde(default)]
    pub subjects: SubjectConfig,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            enabled: default_nats_enabled(),
            servers: default_nats_servers(),
            connect_timeout_seconds: default_nats_connect_timeout_seconds(),
            max_reconnects: default_nats_max_reconnects(),
            reconnect_delay_ms: default_nats_reconnect_delay_ms(),
            auth: NatsAuthConfig::default(),
            jetstream: JetStreamConfig::default(),
            subjects: SubjectConfig::default(),
        }
    }
}

/// NATS authentication configuration.
///
/// Supports multiple authentication methods for NATS connections.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::{NatsAuthConfig, NatsAuthMethod};
///
/// // No authentication (development)
/// let no_auth = NatsAuthConfig {
///     method: NatsAuthMethod::None,
///     ..Default::default()
/// };
///
/// // Username/password authentication
/// let user_pass_auth = NatsAuthConfig {
///     method: NatsAuthMethod::UserPass,
///     username: Some("robot-gateway".to_string()),
///     password: Some("secure-password".to_string()),
///     ..Default::default()
/// };
///
/// // Token-based authentication
/// let token_auth = NatsAuthConfig {
///     method: NatsAuthMethod::Token,
///     token: Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string()),
///     ..Default::default()
/// };
///
/// // Credentials file authentication (NATS 2.0+)
/// let creds_auth = NatsAuthConfig {
///     method: NatsAuthMethod::Credentials,
///     credentials_file: Some("/etc/robotorq/service.creds".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsAuthConfig {
    /// Authentication method to use.
    #[serde(default)]
    pub method: NatsAuthMethod,

    /// Username for user/password authentication.
    #[serde(default)]
    pub username: Option<String>,

    /// Password for user/password authentication.
    #[serde(default)]
    pub password: Option<String>,

    /// Path to credentials file (NATS 2.0+).
    #[serde(default)]
    pub credentials_file: Option<String>,

    /// NATS token for token-based authentication.
    #[serde(default)]
    pub token: Option<String>,
}

impl Default for NatsAuthConfig {
    fn default() -> Self {
        Self {
            method: NatsAuthMethod::None,
            username: None,
            password: None,
            credentials_file: None,
            token: None,
        }
    }
}

/// NATS authentication methods.
///
/// Defines the available authentication mechanisms for connecting to NATS servers.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::NatsAuthMethod;
///
/// // No authentication required (insecure, development only)
/// let none = NatsAuthMethod::None;
///
/// // Username and password authentication
/// let user_pass = NatsAuthMethod::UserPass;
///
/// // NATS credentials file (recommended for production)
/// let credentials = NatsAuthMethod::Credentials;
///
/// // NATS token authentication
/// let token = NatsAuthMethod::Token;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NatsAuthMethod {
    /// No authentication
    #[default]
    None,
    /// Username and password
    UserPass,
    /// NATS credentials file
    Credentials,
    /// NATS token
    Token,
}

/// JetStream configuration for durable messaging.
///
/// Configures NATS JetStream for persistent message storage and streaming.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::{JetStreamConfig, StreamDefaultsConfig, ConsumerDefaultsConfig};
///
/// // Basic JetStream configuration
/// let basic_jetstream = JetStreamConfig {
///     enabled: true,
///     stream_defaults: StreamDefaultsConfig {
///         max_messages: 100_000,
///         max_bytes: 100 * 1024 * 1024, // 100MB
///         ..Default::default()
///     },
///     consumer_defaults: ConsumerDefaultsConfig {
///         max_ack_pending: 100,
///         ack_wait_seconds: 60,
///         max_deliver: 5,
///     },
/// };
///
/// // High-throughput configuration
/// let high_throughput_jetstream = JetStreamConfig {
///     enabled: true,
///     stream_defaults: StreamDefaultsConfig {
///         max_messages: 1_000_000,
///         max_bytes: 10 * 1024 * 1024 * 1024, // 10GB
///         ..Default::default()
///     },
///     consumer_defaults: ConsumerDefaultsConfig {
///         max_ack_pending: 10_000,
///         ack_wait_seconds: 300,
///         max_deliver: 10,
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JetStreamConfig {
    /// Whether JetStream is enabled.
    ///
    /// When enabled, services will create and use JetStream streams for durable messaging.
    #[serde(default = "default_jetstream_enabled")]
    pub enabled: bool,

    /// Default stream configuration.
    ///
    /// Base configuration applied to all JetStream streams created by services.
    #[serde(default)]
    pub stream_defaults: StreamDefaultsConfig,

    /// Consumer configuration.
    ///
    /// Default settings for JetStream consumers (subscribers).
    #[serde(default)]
    pub consumer_defaults: ConsumerDefaultsConfig,
}

impl Default for JetStreamConfig {
    fn default() -> Self {
        Self {
            enabled: default_jetstream_enabled(),
            stream_defaults: StreamDefaultsConfig::default(),
            consumer_defaults: ConsumerDefaultsConfig::default(),
        }
    }
}

/// Default configuration for JetStream streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDefaultsConfig {
    /// Maximum messages per stream.
    ///
    /// Limits total messages stored in the stream. Older messages are discarded.
    #[serde(default = "default_stream_max_messages")]
    pub max_messages: i64,

    /// Maximum bytes per stream.
    ///
    /// Limits total storage used by the stream in bytes.
    #[serde(default = "default_stream_max_bytes")]
    pub max_bytes: i64,

    /// Message retention policy.
    #[serde(default)]
    pub retention: StreamRetentionPolicy,

    /// Storage type for the stream.
    #[serde(default)]
    pub storage: StreamStorageType,
}

impl Default for StreamDefaultsConfig {
    fn default() -> Self {
        Self {
            max_messages: default_stream_max_messages(),
            max_bytes: default_stream_max_bytes(),
            retention: StreamRetentionPolicy::default(),
            storage: StreamStorageType::default(),
        }
    }
}

/// Default configuration for JetStream consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerDefaultsConfig {
    /// Maximum number of unacknowledged messages.
    ///
    /// Limits how many messages can be in-flight before blocking delivery.
    #[serde(default = "default_consumer_max_ack_pending")]
    pub max_ack_pending: i64,

    /// Acknowledgement wait time in seconds.
    ///
    /// How long to wait for message acknowledgement before redelivery.
    #[serde(default = "default_consumer_ack_wait_seconds")]
    pub ack_wait_seconds: i64,

    /// Maximum delivery attempts.
    ///
    /// Number of times to redeliver unacknowledged messages before giving up.
    #[serde(default = "default_consumer_max_deliver")]
    pub max_deliver: i64,
}

impl Default for ConsumerDefaultsConfig {
    fn default() -> Self {
        Self {
            max_ack_pending: default_consumer_max_ack_pending(),
            ack_wait_seconds: default_consumer_ack_wait_seconds(),
            max_deliver: default_consumer_max_deliver(),
        }
    }
}

/// Stream retention policies.
///
/// Controls how messages are retained in JetStream streams.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::StreamRetentionPolicy;
///
/// // Keep messages until stream limits are reached
/// let limits = StreamRetentionPolicy::Limits;
///
/// // Keep messages until all consumers have processed them
/// let interest = StreamRetentionPolicy::Interest;
///
/// // Keep messages until explicitly acknowledged (work queue pattern)
/// let work_queue = StreamRetentionPolicy::WorkQueue;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamRetentionPolicy {
    /// Keep all messages until limits are reached
    #[default]
    Limits,
    /// Keep messages for a specific duration
    Interest,
    /// Keep messages until explicitly deleted
    WorkQueue,
}

/// Stream storage types.
///
/// Defines where JetStream streams store their messages.
///
/// # Examples
///
/// ```rust
/// use commons::util::config::services::nats::StreamStorageType;
///
/// // File-based storage (persistent, survives restarts)
/// let file_storage = StreamStorageType::File;
///
/// // Memory-based storage (fast, but lost on restart)
/// let memory_storage = StreamStorageType::Memory;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamStorageType {
    /// File-based storage (persistent)
    #[default]
    File,
    /// Memory-based storage (ephemeral)
    Memory,
}

/// Subject naming configuration.
///
/// Defines the subject hierarchy and naming conventions for inter-service communication.
/// Follows the pattern: `rtq.<service>.<type>.*`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectConfig {
    /// Base prefix for all RoboTorq subjects.
    ///
    /// All subjects start with this prefix for namespacing.
    #[serde(default = "default_subject_prefix")]
    pub prefix: String,

    /// Service-specific subject patterns.
    ///
    /// Defines the subject structure for each service type.
    #[serde(default)]
    pub services: ServiceSubjects,
}

impl Default for SubjectConfig {
    fn default() -> Self {
        Self {
            prefix: default_subject_prefix(),
            services: ServiceSubjects::default(),
        }
    }
}

/// Subject patterns for each service type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceSubjects {
    /// Robot Gateway service subjects.
    #[serde(default)]
    pub gateway: ServiceSubjectPatterns,

    /// Refinery service subjects.
    #[serde(default)]
    pub refinery: ServiceSubjectPatterns,

    /// Mint service subjects.
    #[serde(default)]
    pub mint: ServiceSubjectPatterns,

    /// Vault service subjects.
    #[serde(default)]
    pub vault: ServiceSubjectPatterns,
}

/// Subject patterns for a specific service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSubjectPatterns {
    /// Command subjects for requesting actions.
    #[serde(default = "default_cmd_pattern")]
    pub cmd: String,

    /// Event subjects for publishing state changes.
    #[serde(default = "default_events_pattern")]
    pub events: String,

    /// Query subjects for requesting data.
    #[serde(default = "default_query_pattern")]
    pub query: String,
}

impl Default for ServiceSubjectPatterns {
    fn default() -> Self {
        Self {
            cmd: default_cmd_pattern(),
            events: default_events_pattern(),
            query: default_query_pattern(),
        }
    }
}

// Default value functions

/// Returns the default NATS enabled state.
///
/// Returns `true` to enable NATS connectivity by default, supporting
/// distributed service communication.
fn default_nats_enabled() -> bool { true }

/// Returns the default NATS server URLs.
///
/// Returns `["nats://localhost:4222"]` for local development.
/// In production, this should be configured with cluster URLs.
fn default_nats_servers() -> Vec<String> { vec!["nats://localhost:4222".to_string()] }

/// Returns the default NATS connection timeout.
///
/// Returns `10` seconds as a reasonable timeout for establishing
/// initial connections to NATS servers.
fn default_nats_connect_timeout_seconds() -> u64 { 10 }

/// Returns the default maximum reconnection attempts.
///
/// Returns `60` attempts, allowing approximately 1 minute of reconnection
/// attempts with the default delay before giving up.
fn default_nats_max_reconnects() -> usize { 60 }

/// Returns the default reconnection delay.
///
/// Returns `1000` milliseconds (1 second) as the base delay between
/// reconnection attempts, which may be increased exponentially.
fn default_nats_reconnect_delay_ms() -> u64 { 1000 }

/// Returns the default JetStream enabled state.
///
/// Returns `true` to enable JetStream by default for durable messaging
/// and event streaming capabilities.
fn default_jetstream_enabled() -> bool { true }

/// Returns the default maximum messages per stream.
///
/// Returns `1,000,000` messages as a reasonable default for most streams.
/// This prevents unbounded growth while allowing sufficient history.
fn default_stream_max_messages() -> i64 { 1_000_000 }

/// Returns the default maximum bytes per stream.
///
/// Returns `1GB` as a reasonable storage limit for stream data.
/// This balances storage requirements with retention needs.
fn default_stream_max_bytes() -> i64 { 1_073_741_824 } // 1GB

/// Returns the default maximum unacknowledged messages per consumer.
///
/// Returns `1000` messages to limit memory usage and prevent
/// consumers from being overwhelmed with in-flight messages.
fn default_consumer_max_ack_pending() -> i64 { 1000 }

/// Returns the default acknowledgement wait time.
///
/// Returns `30` seconds for consumers to acknowledge message processing.
/// Messages not acknowledged within this time will be redelivered.
fn default_consumer_ack_wait_seconds() -> i64 { 30 }

/// Returns the default maximum delivery attempts.
///
/// Returns `3` attempts before giving up on message delivery.
/// This prevents infinite redelivery of problematic messages.
fn default_consumer_max_deliver() -> i64 { 3 }

/// Returns the default subject prefix.
///
/// Returns `"rtq"` (RoboTorq) as the base prefix for all subjects,
/// providing namespacing for the distributed system.
fn default_subject_prefix() -> String { "rtq".to_string() }

/// Returns the default command subject pattern.
///
/// Returns `"cmd.*"` for command subjects that request actions.
/// The `*` allows service-specific command routing.
fn default_cmd_pattern() -> String { "cmd.*".to_string() }

/// Returns the default events subject pattern.
///
/// Returns `"events.*"` for event subjects that publish state changes.
/// The `*` allows event type-specific routing.
fn default_events_pattern() -> String { "events.*".to_string() }

/// Returns the default query subject pattern.
///
/// Returns `"query.*"` for query subjects that request data.
/// The `*` allows query type-specific routing.
fn default_query_pattern() -> String { "query.*".to_string() }