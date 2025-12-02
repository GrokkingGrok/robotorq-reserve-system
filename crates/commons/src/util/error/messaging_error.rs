use thiserror::Error;

/// Messaging subsystem errors (NATS, JetStream, connectivity).
#[derive(Debug, Error)]
pub enum MessagingError {
    /// Underlying client error (e.g. async-nats).
    #[error("NatsError: {0}")]
    Nats(String),

    /// Failure publishing a message.
    #[error("PublishError: {0}")]
    Publish(String),

    /// Failure subscribing or handling subscriptions.
    #[error("SubscribeError: {0}")]
    Subscribe(String),

    /// Other messaging related errors.
    #[error("Unknown messaging error: {0}")]
    Other(String),
}

#[cfg(feature = "messaging")]
impl From<async_nats::Error> for MessagingError {
    fn from(e: async_nats::Error) -> Self {
        MessagingError::Nats(format!("async-nats error: {}", e))
    }
}
