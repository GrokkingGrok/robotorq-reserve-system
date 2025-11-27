use commons::logging::{init_logging_pretty};
use tracing::info;

fn main() {
    // Initialize human-friendly colored logs for local dev.
    init_logging_pretty("info");

    info!(component = "robot-gateway", "gateway starting up");

    // Placeholder: real startup logic goes here.
    info!(component = "robot-gateway", "ready");
}
