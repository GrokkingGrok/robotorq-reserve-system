# Code Style & Standards

Unified guidance for Go, Rust, Python (tests), and Markdown across RoboTorq.

## Global Principles
- Deterministic & testable: avoid hidden globals; prefer explicit dependency injection.
- Fail loudly & early in constructors; fail gracefully in runtime loops.
- Structured logging everywhere (no `fmt.Printf` / bare `println`).
- Observability is part of definition of done.

## Go
- Logging: use `slog` with contextual fields (ids, counts, durations).
- Errors: wrap with context (`fmt.Errorf("ingot validate: %w", err)`). No silent ignores.
- Concurrency: prefer channels or `sync.Cond` to busy‑waiting. Guard shared slices/maps with `sync.Mutex`.
- Panics: only in truly unrecoverable init states; convert to errors in service loops.
- Testing: table‑driven for pure functions; concurrency tests validate absence of races (`go test -race`).
- Layout: keep public types/functions at top, private helpers below.

## Rust (Incoming)
- Error Handling: use `Result<T, E>`; implement `thiserror::Error` for custom domains.
- Async: prefer `tokio` tasks with cancellation tokens; no detached tasks without join strategy.
- Logging: use `tracing` with structured spans; record timing for critical paths.
- Clippy: run `cargo clippy --all-targets -- -D warnings` before PR.
- Unsafe: forbidden unless reviewed in RFC; require justification & tests.

## Python (Tests / Scripts)
- Keep logic minimal; heavy lifting resides in services.
- Use `asyncio` for NATS interactions; avoid arbitrary `sleep` without reason (wrap in wait helpers).
- No production secrets or credentials in scripts.

## Merkle & Crypto Code
- Deterministic ordering: document leaf order, hashing algorithm, and encoding scheme.
- Add test vectors: fixed inputs produce known outputs committed in tests.
- No custom crypto primitives; use vetted libraries only.

## Metrics
- Naming: `service_subsystem_action_total` for counters; `*_seconds` for histograms.
- Cardinality: avoid high‑cardinality labels (e.g., user IDs) in hot paths.
- Always increment success+failure counters; latency histograms around core operations.

## Logging Fields (Guideline)
- Common: `component`, `ingot_id`, `batch_id`, `unit_count`, `duration_ms`, `error`.
- Avoid logging full raw payloads; log hashes/ids for correlation.

## Directory & File Conventions
- `internal/` for non‑public packages.
- Tests colocated (`*_test.go`) for Go; `tests/` root for integration/e2e Python.
- Architecture docs: `ARCHITECTURE.md` inside service directory.

## Naming
- Structs / Types: `PascalCase`.
- Private helpers: concise `camelCase`.
- Constants: `ALL_CAPS` only for exported sentinel values; prefer `CamelCase` otherwise.

## Comments
- Focus on WHY (intent, invariant) vs WHAT (the code spells that out).
- Exported functions/types: brief doc comment suitable for generated docs.
- TODO format: `TODO(<feature|scope>): description` ; `FIXME(issue-#):` for known defects.

## Formatting & Tooling
| Language | Formatter | Command |
|----------|-----------|---------|
| Go | gofmt / goimports | `go fmt ./...` |
| Rust | rustfmt | `cargo fmt --all` |
| Python | black (if needed) | `black tests/` |
| Markdown | Prettier (optional) | `npx prettier --write '**/*.md'` |

## Performance Considerations
- Avoid allocating inside tight loops; reuse buffers where safe.
- Profile hotspots before micro‑optimizing (`go test -bench`, `cargo bench`).
- Use streaming / incremental hashing for large batch operations.

## Error Classification
- Transient (retryable): network hiccups, temporary NATS failures.
- Permanent: invalid merkle path, malformed ingot structure.
- Use distinct log levels (warn vs error) accordingly.

## Return Early Pattern
```
if err != nil {
    return fmt.Errorf("stage2 assemble: %w", err)
}
```
Reduces nesting and improves readability.

## PR Hygiene
- No commented‑out code fragments.
- No stray debug prints.
- Ensure tests cover positive & negative paths.
- Benchmarks included for significant perf changes.

## Backward Compatibility
- Document any breaking change in PR body under "Breaking Changes".
- Provide migration steps (data format, environment variables).

## Security
- Validate lengths & bounds on external numeric inputs.
- Reject oversized payloads early.
- Prefer constant‑time compares for sensitive hashes when comparing authentication tokens.

---
Consistent style accelerates review, reduces defects, and preserves the integrity of a physics‑anchored monetary system.
