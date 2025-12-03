use std::time::Duration;
use commons::util::logging::{init_prod_tracing, OtlpConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure and initialize the commons logging with OTLP exporter.
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    let otlp_cfg = OtlpConfig { endpoint };
    // json=true for structured logs, default level "info", no rolling dir
    let _ = init_prod_tracing(true, "info", None, Some(otlp_cfg));

    // Emit a span and an event that the collector should receive.
    // Add a searchable attribute `example_id` to make collector verification easy.
    let span = tracing::info_span!("emit_traces_example", example = "emit_traces", example_id = "robotorq_emit_traces_test_001");
    let _enter = span.enter();
    tracing::info!(message = "hello otlp from example");

    // Give the exporter a moment to flush the batch
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Shutdown tracer provider so exporter flushes (no-op when `otlp` feature not enabled)
    commons::util::logging::shutdown_tracer_provider();
    Ok(())
}
