use commons::util::logging::init_logging_pretty;
use commons::util::error::{InvariantError, logging_error::LoggingError};
use commons::services::robot_gateway::metrics::RobotGatewayMetrics;
use commons::services::robot_gateway::RobotGateway;
use commons::services::http::HttpServerConfig;
use commons::util::config::port_mapping::ROBOT_GATEWAY_PORT;
use commons::types::ids::RobotId;
// HTTP types are configured via HttpServerConfig; no direct imports needed here
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
    let cfg = HttpServerConfig::local_defaults(ROBOT_GATEWAY_PORT);
    let addr = format!("{}:{}", cfg.service.address, cfg.service.port);
    let gw = RobotGateway::single(RobotId::new())
        .with_metrics(gw_metrics)
        .with_http_config(cfg);

    info!(component = "robot-gateway", %addr, "starting http server");
    match gw.start_http_server() {
        Ok(handle) => {
            info!(component = "robot-gateway", %addr, "http server ready");
            let _ = handle.join();
        }
        Err(err) => {
            error!(component = "robot-gateway", error = %err, "failed to start http server");
            std::process::exit(1);
        }
    }
}
