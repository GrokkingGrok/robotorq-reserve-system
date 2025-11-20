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

    // Load configuration
    let config = Config::from_file("config.yaml")
        .unwrap_or_else(|e| {
            error!("Failed to load config: {}", e);
            error!("Using default configuration");
            Config::default()
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

    // Start metrics server
    tokio::spawn(async {
        if let Err(e) = printer::metrics::start_metrics_server(9091).await {
            error!("Metrics server error: {}", e);
        }
    });

    info!("🚀 Starting status reporting loop...");
    info!("");

    // Run main service loop
    service.run().await?;

    Ok(())
}
