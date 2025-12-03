# OTLP Local Setup & Example Smoke Test

This document describes how to run the OpenTelemetry Collector locally, run the `emit_traces` example that ships with the workspace, and use the provided pre-check script used by CI.

Prerequisites
- Docker and Docker Compose installed
- Rust toolchain (to build and run the example) — `cargo` on PATH
- From Windows PowerShell, run commands from the repository root, e.g. `C:\Users\Jon\Documents\Project-Asimov\robotorq-reserve-system`

Start the local collector
1. From the repo root run:

```powershell
docker compose -f .\ci\otlp-collector\docker-compose.yml up -d
```

2. Verify the grpc OTLP port is reachable on `127.0.0.1:4317`.

Run the example that emits traces
1. Build and run the example with the OTLP feature enabled:

```powershell
cargo run --bin emit_traces --features otlp --release
```

2. The example emits a span with attribute `example_id = "robotorq_emit_traces_test_001"`.

Use the pre-check script (local / CI)
1. The repo includes `scripts/run_otlp_precheck.py` which performs these steps:
   - starts the collector via `docker compose -f ci/otlp-collector/docker-compose.yml up -d`
   - waits until `127.0.0.1:4317` is reachable
   - runs the example `cargo run --bin emit_traces --features otlp`
   - collects `docker compose logs` for the collector into `otel-collector.log`
   - searches the logs and/or the example stdout for evidence of ingestion (example id, span name, or TracesExporter summary)
   - tears down the collector

2. Example invocation (PowerShell):

```powershell
python .\scripts\run_otlp_precheck.py --post-wait 8
```

CI notes
- The GitHub Actions workflow `.github/workflows/otlp-e2e.yml` runs the same script to perform an end-to-end smoke test in CI and uploads `otel-collector.log` as an artifact.
- The workflow is currently a manual/dispatch job on branch `rewrite-core` to keep changes reviewable before merging to `release/v0`.

Troubleshooting
- If the example runs but the collector logs do not show traces, increase `--post-wait` to allow exporter batching.
- Use `docker compose -f .\ci\otlp-collector\docker-compose.yml logs --no-color --tail=1000` to inspect collector logs.

Maintenance
- Keep `ci/otlp-collector/docker-compose.yml` updated to a supported `otel/opentelemetry-collector-contrib` release.
- Avoid shipping long-lived or production collection using this small local compose — this is purely a dev/CI smoke-test harness.

Revision
- 2025-12-03: Initial guide added.
