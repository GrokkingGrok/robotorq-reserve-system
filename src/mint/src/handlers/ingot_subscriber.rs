use async_nats::Client;
use futures_util::stream::StreamExt;
use crate::models::token_torq_ingot::TokenTorqIngot;
use crate::engine::ingot_processor::IngotProcessor;
use crate::metrics::MintMetrics;
use std::sync::Arc;
use tracing::info;

/// IngotSubscriber: Subscribes to ingot messages from NATS and forwards to processor.
pub struct IngotSubscriber {
    nats_client: Client,
    processor: Arc<IngotProcessor>,
    metrics: Arc<MintMetrics>,
}

impl IngotSubscriber {
    pub fn new(
        nats_client: Client,
        processor: Arc<IngotProcessor>,
        metrics: Arc<MintMetrics>,
    ) -> Self {
        Self {
            nats_client,
            processor,
            metrics,
        }
    }

    /// Start subscribing to ingot messages.
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut subscriber = self.nats_client.subscribe("mint.ingots").await?;
        info!("Subscribed to mint.ingots");

        let processor = Arc::clone(&self.processor);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                let processor_clone = Arc::clone(&processor);
                let metrics_clone = Arc::clone(&metrics);

                tokio::spawn(async move {
                    match serde_json::from_slice::<TokenTorqIngot>(&message.payload) {
                        Ok(ingot) => {
                            metrics_clone.inc_ingots_received();

                            if let Err(e) = processor_clone.process_ingot_async(ingot).await {
                                tracing::error!("Failed to process ingot: {}", e);
                                metrics_clone.inc_processing_errors();
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to deserialize ingot: {}", e);
                            metrics_clone.inc_processing_errors();
                        }
                    }
                });
            }
        });

        Ok(())
    }
}