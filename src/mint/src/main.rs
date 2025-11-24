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
mod archive;
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

    // Start metrics & health server (simple manual HTTP parsing; upgrade later to axum if needed)
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use std::time::{SystemTime, UNIX_EPOCH};

        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        info!("Metrics/Health server listening on 0.0.0.0:8080");

        // Uptime ticker: increment metric every second
        let metrics_uptime = Arc::clone(&metrics_clone);
        tokio::spawn(async move {
            loop {
                metrics_uptime.inc_uptime();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let metrics = Arc::clone(&metrics_clone);
            tokio::spawn(async move {
                let mut buf = [0; 2048];
                let n = match socket.read(&mut buf).await { Ok(n) => n, Err(_) => return };
                let request = String::from_utf8_lossy(&buf[..n]);

                let method_line = request.lines().next().unwrap_or("");
                let is_get = method_line.starts_with("GET ");
                let path = if is_get { method_line.split_whitespace().nth(1).unwrap_or("/") } else { "/" };

                let response = match path {
                    "/metrics" => {
                        let body = metrics.encode();
                        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n{}", body.len(), body)
                    }
                    "/health" => {
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        let body = serde_json::json!({
                            "status": "ok",
                            "timestamp": now,
                            "uptime_seconds": metrics.service_uptime_seconds.get() as u64,
                            "last_batch_stake": metrics.batch_last_stake.get(),
                            "last_batch_jouletorq": metrics.batch_last_jouletorq.get(),
                            "certificate_queue_size": metrics.certificate_queue_size.get(),
                        }).to_string();
                        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n{}", body.len(), body)
                    }
                    _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string(),
                };
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    // Create channels for inter-component communication
    let (certificate_sender, _certificate_receiver) = mpsc::channel::<models::robotorq_certificate::RoboTorqCertificate>(100);
    let (batch_sender, batch_receiver) = mpsc::channel::<models::robotorq_batch::RoboTorqBatch>(10);

    // Create archive (in-memory) if enabled
    use crate::archive::MintArchive;
    let archive = MintArchive::new(config.enable_archive);

    // Create processing components
    let ingot_processor = Arc::new(engine::ingot_processor::IngotProcessor::new(
        certificate_sender,
        Arc::clone(&proof_engine),
        Arc::clone(&metrics),
        Arc::clone(&archive),
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