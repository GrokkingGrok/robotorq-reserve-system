# Contributing

Thanks for contributing! This file contains a short developer note about local OTLP tracing setup used by maintainers.

## OTLP / Tracing (Local smoke tests)

We maintain a lightweight, feature-gated OTLP export in `crates/commons` for local development and CI smoke tests. To exercise tracing locally:

- Start the local OpenTelemetry Collector (from repo root):

```powershell
docker compose -f .\ci\otlp-collector\docker-compose.yml up -d
```

- Build and run the example that emits a test span (enable OTLP at runtime via Cargo features):

```powershell
cargo run --bin emit_traces --features otlp --release
```

- Optional: run the provided pre-check script which starts the collector, runs the example, captures collector logs, and tears down the collector:

```powershell
python .\scripts\run_otlp_precheck.py --post-wait 8
```

Docs:
- Local OTLP setup and troubleshooting: `docs/OTLP_LOCAL_SETUP.md`
- Message envelope & propagation guidance: `docs/MESSAGE_ENVELOPE.md`

Notes:
- The OTLP CI workflow exists on the `rewrite-core` branch for testing but is intentionally not enabled on the default branch (`release/v0`) until this feature is production-ready.
- To enable OTLP for other crates that depend on `commons`, set the dependency with features in your `Cargo.toml`, e.g.:

```toml
[dependencies]
commons = { path = "../commons", features = ["otlp"], default-features = false }
```

If you hit problems with the collector or missing traces, consult `docs/OTLP_LOCAL_SETUP.md` for troubleshooting tips (increasing `--post-wait`, inspecting `docker compose logs`, etc.).
