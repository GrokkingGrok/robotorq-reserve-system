use commons::services::http::{HttpServer, HttpServerConfig, RoboTorqService};
use commons::types::ids::RobotId;
use commons::util::config::load_robotorq_config;
use commons::util::error::{InvariantError, logging_error::LoggingError};
use commons::util::logging::init_logging_pretty;
use robot_gateway::RobotGateway;
use robot_gateway::metrics::RobotGatewayMetrics;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize human-friendly colored logs for local dev.
    if let Err(e) = init_logging_pretty("info") {
        let err: InvariantError = LoggingError::from(e).into();
        error!(component = "robot-gateway", error = %err, "startup failed: logging init");
        std::process::exit(1);
    }

    info!(component = "robot-gateway", "gateway starting up");

    // Load configuration using unified loader
    let config = match load_robotorq_config(None) {
        Ok(config) => config,
        Err(e) => {
            error!(component = "robot-gateway", error = %e, "failed to load configuration");
            std::process::exit(1);
        }
    };

    info!(component = "robot-gateway", mode = ?config.mode, "configuration loaded");

    // Create HTTP server configuration from loaded config
    let http_config = HttpServerConfig::local_defaults(config.ports.robot_gateway_port);
    let addr = format!(
        "{}:{}",
        http_config.service.address, http_config.service.port
    );

    // Create the robot gateway service
    let metrics = RobotGatewayMetrics::new("robot_gateway");
    let mut gateway = RobotGateway::single(RobotId::new()).with_metrics(metrics);

    // Initialize the service with configuration
    if let Err(e) = gateway.initialize(&config).await {
        error!(component = "robot-gateway", error = %e, "failed to initialize service");
        std::process::exit(1);
    }

    // Start the service
    if let Err(e) = gateway.start().await {
        error!(component = "robot-gateway", error = %e, "failed to start service");
        std::process::exit(1);
    }

    // Wrap in Arc for HTTP server
    let gateway_arc = Arc::new(gateway);

    // Create and start the HTTP server
    let server = HttpServer::new(Arc::clone(&gateway_arc), http_config);
    info!(component = "robot-gateway", %addr, "starting HTTP server");

    // Set up graceful shutdown handling
    let shutdown_signal = async {
        // Wait for Ctrl+C signal
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        info!(component = "robot-gateway", "received shutdown signal");
    };

    // Start the server with graceful shutdown
    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                error!(component = "robot-gateway", error = %e, "HTTP server error");
                std::process::exit(1);
            }
        }
        _ = shutdown_signal => {
            info!(component = "robot-gateway", "initiating graceful shutdown");
        }
    }

    // Perform graceful shutdown of the service
    info!(component = "robot-gateway", "stopping service");
    if let Err(e) = gateway_arc.stop().await {
        error!(component = "robot-gateway", error = %e, "error stopping service");
    }

    info!(component = "robot-gateway", "shutting down service");
    if let Err(e) = gateway_arc.shutdown().await {
        error!(component = "robot-gateway", error = %e, "error shutting down service");
    }

    info!(component = "robot-gateway", "shutdown complete");

    Ok(())
}
