use commons::util::logging::init_prod_tracing;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize JSON logging for the demo
    init_prod_tracing(true, "info", None, None)?;

    tracing::info!(
        service = "logging-demo",
        component = "demo",
        "service.started"
    );

    // Give the logger a moment to flush on slower CI machines.
    std::thread::sleep(Duration::from_millis(50));

    Ok(())
}
