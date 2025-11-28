use commons::util::logging::init_logging_pretty;
use commons::util::error::{InvariantError, logging_error::LoggingError};
use commons::services::robot_gateway::metrics::RobotGatewayMetrics;
use commons::services::robot_gateway::RobotGateway;
use commons::services::http::{HttpServerConfig, request_graceful_shutdown};
use ctrlc;
use commons::util::config::parameters::load_ports_config_from_default;
use commons::types::ids::RobotId;
use tracing::{info, error};

fn main() {
    // Initialize human-friendly colored logs for local dev.
    if let Err(e) = init_logging_pretty("info") {
        let err: InvariantError = LoggingError::from(e).into();
        error!(component = "robot-gateway", error = %err, "startup failed: logging init");
        std::process::exit(1);
    }

    info!(component = "robot-gateway", "gateway starting up");

    // Build gateway with metrics and HTTP config
    let gw_metrics = RobotGatewayMetrics::new("robot_gateway");
    let ports = load_ports_config_from_default();
    info!(component = "robot-gateway", port = ports.robot_gateway_port, "port.toml loaded, ");
    let cfg = HttpServerConfig::local_defaults(ports.robot_gateway_port);
    let addr = format!("{}:{}", cfg.service.address, cfg.service.port);
    let gw = RobotGateway::single(RobotId::new())
        .with_metrics(gw_metrics)
        .with_http_config(cfg.clone());

    info!(component = "robot-gateway", %addr, "starting http server");
        let handle = match gw.start_http_server() {
            Ok(handle) => {
                info!(component = "robot-gateway", %addr, "http server ready");
                handle
            }
            Err(err) => {
                error!(component = "robot-gateway", error = %err, "failed to start http server");
                std::process::exit(1);
            }
        };

        // Graceful shutdown on Ctrl+C (SIGINT)
        let service = cfg.service.clone();
        ctrlc::set_handler(move || {
            info!(component = "robot-gateway", %addr, "CTRL+C received, http server shutting down");
            let _ = request_graceful_shutdown(&service);
        }).expect("failed to set Ctrl+C handler");

        // Block until server thread exits
        let _ = handle.join();
}
