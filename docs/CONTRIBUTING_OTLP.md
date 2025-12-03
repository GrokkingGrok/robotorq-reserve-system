# OTLP / OpenTelemetry Notes (Local dev & CI)

This short guide explains how to exercise and verify OTLP trace export locally and in CI for the RoboTorq template.

Summary
- The commons logging/tracing initializer supports an OTLP exporter behind a feature gate.
- Use the local collector (provided under `ci/otlp-collector/`) to verify traces locally.

Local quick-start
1. Start the OpenTelemetry Collector (local):

```powershell
# from repository root
docker compose -f ci/otlp-collector/docker-compose.yml up -d
```

2. Point the service at the collector. You can either set the environment variable(s) your tracing init expects (example):

```powershell
$env:OTEL_EXPORTER_OTLP_ENDPOINT = 'http://localhost:4318'
$env:RUST_LOG = 'info'
# then run the service or an example that calls init_prod_tracing(json=true, otlp=Some(...))
cargo run -p examples --example minimal_service
```

3. The collector in `ci/otlp-collector/config.yaml` is configured with a `logging` exporter so traces will be printed to the collector logs. To view them:

```powershell
docker logs $(docker ps --filter "name=otel-collector" -q) --follow
```

CI Prototype (GitHub Actions)
- A small prototype workflow `/.github/workflows/otlp-ci.yml` is included which demonstrates starting the collector in CI and ensuring the collector process launches successfully. Depending on your CI policy you can expand that workflow to run a lightweight tracing example that emits spans and validates collector logs.

Notes & recommendations
- The repo uses a feature-gated OTLP exporter to avoid pulling heavy tracing deps in normal build profiles. The exact feature name in `crates/commons` is `otlp` — enable it in a downstream `Cargo.toml` dependency with:

```toml
commons = { path = "../commons", features = ["otlp"], default-features = false }
```

Alternatively the `crates/examples` package includes a small example binary `emit_traces` that uses `opentelemetry`, `opentelemetry-otlp`, and `tracing-opentelemetry` directly; the CI prototype runs that example to validate collector reception.
- For secure/production OTLP endpoints use TLS and credentialed endpoints (collector/OTLP exporter configuration beyond the prototype is out-of-scope for this guide).
- Long-running collector + integration tests increase CI runtime and may require dedicated runners. Use the prototype workflow as a starting point.

Files added for local/CI testing:
- `ci/otlp-collector/config.yaml` — collector pipeline (receivers -> logging exporter)
- `ci/otlp-collector/docker-compose.yml` — quick-start compose file
- `scripts/run_otlp_collector.ps1` — small helper to run the collector locally
- `.github/workflows/otlp-ci.yml` — prototype workflow to start collector in CI

If you'd like, I can expand the CI workflow to run a small Rust binary that emits traces and asserts that the collector received them (needs an example harness). Say the word and I'll add it.
