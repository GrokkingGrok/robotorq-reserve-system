# Observability & OTLP (Quick Start)

This document explains how to enable OpenTelemetry (OTLP) for local testing and how to set recommended `RUST_LOG` defaults for development and CI.

## Enabling OTLP locally

The commons crate ships with feature-gated OTLP support under the `otlp` cargo feature. To run locally with an OTLP collector:

1. Start a local OpenTelemetry Collector (Docker):

```powershell
# from repo root
cat > otel-config.yaml <<'EOF'
receivers:
  otlp:
    protocols:
      grpc:
      http:
exporters:
  logging:
    logLevel: debug
  prometheus:
    endpoint: "0.0.0.0:8888"
service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [logging]
    metrics:
      receivers: [otlp]
      exporters: [prometheus, logging]
EOF

docker run -d --rm --name otel-collector -p 4317:4317 -p 8888:8888 \
  -v "$PWD/otel-config.yaml:/etc/otel-config.yaml" \
  otel/opentelemetry-collector-contrib:0.76.0 --config /etc/otel-config.yaml
```

2. Set the OTLP endpoint for your process and enable the `otlp` feature when building/running:

```powershell
$env:OTEL_EXPORTER_OTLP_ENDPOINT = 'http://127.0.0.1:4317'
cargo run -p your-service --features otlp
```

Notes:
- The collector config above routes traces to a `logging` exporter and exposes metrics at `:8888` (Prometheus endpoint).
- The `otlp` feature is feature-gated to avoid pulling heavy dependencies into CI by default.

## RUST_LOG defaults

Recommended logging levels for common workflows:

- Local development: `RUST_LOG=debug` — gives verbose output for troubleshooting.
- CI / smoke tests: `RUST_LOG=info` with JSON output enforced for structured log assertions.
- Production: `RUST_LOG=info` and enable OTLP exporter via feature and environment when a collector is deployed.

## CI OTLP E2E

A manual workflow (`.github/workflows/otlp-e2e.yml`) is included to run the OpenTelemetry Collector inside CI and execute targeted tests with the `otlp` feature enabled. Use the workflow dispatch UI to run it on-demand.

## Troubleshooting

- If you see handshake errors from the printer WebSocket, prefer the `tokio-tungstenite::connect_async(&url)` variant so the client generates a compliant handshake.
- If CI shows stale dependency behavior, consider invalidating the GitHub Actions cache or updating the cache key to include the `Cargo.lock` or workflow run id.
