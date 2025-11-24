use async_nats::Client;
use tokio::sync::mpsc;
use crate::models::robotorq_batch::RoboTorqBatch;
use crate::metrics::MintMetrics;
use std::sync::Arc;
use tracing::{info, error};

/// BatchPublisher: Publishes completed batches to NATS.
pub struct BatchPublisher {
    nats_client: Client,
    metrics: Arc<MintMetrics>,
}

impl BatchPublisher {
    pub fn new(nats_client: Client, metrics: Arc<MintMetrics>) -> Self {
        Self {
            nats_client,
            metrics,
        }
    }

    /// Start the publishing loop.
    pub async fn start(&self, mut batch_receiver: mpsc::Receiver<RoboTorqBatch>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Batch publisher started");

        while let Some(batch) = batch_receiver.recv().await {
            match serde_json::to_vec(&batch) {
                Ok(payload) => {
                    match self.nats_client.publish("distodam.batches", payload.into()).await {
                        Ok(_) => {
                            info!("Published batch: {}", batch.batch_id);
                        }
                        Err(e) => {
                            error!("Failed to publish batch {}: {}", batch.batch_id, e);
                            self.metrics.inc_nats_publish_errors();
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to serialize batch {}: {}", batch.batch_id, e);
                    self.metrics.inc_processing_errors();
                }
            }
        }

        Ok(())
    }
}