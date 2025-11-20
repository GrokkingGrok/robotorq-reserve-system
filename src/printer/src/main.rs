use anyhow::Result;
use printer::{Config, PrinterService};
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    // Use LocalSet for !Send futures (warp)
    let local = tokio::task::LocalSet::new();
    local.run_until(async_main()).await
}

async fn async_main() -> Result<()> {

    // Load configuration (env vars override file)
    let config = Config::from_file("config.yaml")
        .map(|mut c| {
            c.apply_env_overrides();
            c
        })
        .unwrap_or_else(|e| {
            error!("Failed to load config file: {}", e);
            error!("Using environment variables and defaults");
            Config::from_env()
        });

    // Print startup banner
    info!("╔══════════════════════════════════════════════════╗");
    info!("║     RoboTorq Printer Service v0.1.0             ║");
    info!("╚══════════════════════════════════════════════════╝");
    info!("");
    info!("🖨️  Printer Configuration:");
    info!("   ID: {}", config.printer_id);
    info!("   Model: {}", config.printer_model);
    info!("   Rated Capacity: {}W", config.rated_watts);
    info!("   NATS: {}", config.nats_url);
    info!("   Klipper: {}", config.klipper_url);
    info!("");

    // Initialize service
    let mut service = match PrinterService::new(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("❌ Failed to initialize service: {}", e);
            return Err(e);
        }
    };

    // Load or request certificate
    if let Err(e) = service.initialize().await {
        error!("❌ Failed to initialize certificate: {}", e);
        return Err(e);
    }

    info!("🚀 Starting status reporting loop...");
    info!("");

    // Start metrics server in LocalSet-compatible spawn
    tokio::task::spawn_local(async move {
        if let Err(e) = printer::metrics::start_metrics_server(9091).await {
            error!("Metrics server error: {}", e);
        }
    });

    // Start mock API server if in mock mode
    let service_config = service.config().clone();
    if service_config.mock_mode {
        info!("🎮 Mock mode enabled - starting mock API server on port 9092");
        
        let mock_client = std::sync::Arc::new(printer::MockKlipperClient::new());
        
        tokio::task::spawn_local(async move {
            if let Err(e) = printer::mock_api::start_mock_api_server(9092, mock_client).await {
                error!("Mock API server error: {}", e);
            }
        });
    }

    // Run main service
    service.run().await?;

    Ok(())
}
