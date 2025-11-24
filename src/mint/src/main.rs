use anyhow::Result;
use tracing::info;
use std::sync::Arc;
use tokio::sync::mpsc;

mod config;
mod nats_client;
mod metrics;
mod models;
mod engine;
mod handlers;
// Crypto now supplied by common crate (trait-based); no direct use here

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Rust Mint service");

    // Load config
    let config = Arc::new(config::MintConfig::from_env()?);

    // Initialize metrics
    let metrics = metrics::MintMetrics::new();

    // Initialize proof engine
    let proof_engine = Arc::new(engine::proof_engine::ProofEngine::new(
        Arc::clone(&config),
        Arc::clone(&metrics),
    ));

    info!("Proof engine initialized (crypto: {})", proof_engine.crypto_enabled());

    // Connect to NATS
    let nats_client = nats_client::connect(&config.nats_url).await?;

    info!("Connected to NATS at {}", config.nats_url);

    // Start metrics server
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        info!("Metrics server listening on 0.0.0.0:8080");

        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let metrics = Arc::clone(&metrics_clone);

            tokio::spawn(async move {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);

                let response = if request.contains("GET /metrics") {
                    let body = metrics.encode();
                    format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", body.len(), body)
                } else if request.contains("GET /health") {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK".to_string()
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
                };

                socket.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });

    // Create channels for inter-component communication
    let (certificate_sender, _certificate_receiver) = mpsc::channel::<models::robotorq_certificate::RoboTorqCertificate>(100);
    let (batch_sender, batch_receiver) = mpsc::channel::<models::robotorq_batch::RoboTorqBatch>(10);

    // Create processing components
    let ingot_processor = Arc::new(engine::ingot_processor::IngotProcessor::new(
        certificate_sender,
        Arc::clone(&proof_engine),
        Arc::clone(&metrics),
    ));

    let batcher = Arc::new(engine::batcher::Batcher::new(
        batch_sender,
        config.batch_threshold_seconds as u64,
        Arc::clone(&metrics),
    ));

    // Create handlers
    let ingot_subscriber = handlers::ingot_subscriber::IngotSubscriber::new(
        nats_client.clone(),
        Arc::clone(&ingot_processor),
        Arc::clone(&metrics),
    );

    let batch_publisher = handlers::batch_publisher::BatchPublisher::new(
        nats_client.clone(),
        Arc::clone(&metrics),
    );

    // Start processing tasks
    ingot_subscriber.start().await?;
    tokio::spawn(async move {
        if let Err(e) = batcher.start_batching_loop().await {
            tracing::error!("Batcher error: {}", e);
        }
    });

    tokio::spawn(async move {
        if let Err(e) = batch_publisher.start(batch_receiver).await {
            tracing::error!("Batch publisher error: {}", e);
        }
    });

    // For now, just keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Mint service");

    Ok(())
}