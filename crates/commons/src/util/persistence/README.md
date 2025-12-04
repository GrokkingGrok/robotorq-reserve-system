Persistence module (commons)

This folder contains the persistence abstractions and a small proof-of-concept SQLite driver.

Overview
- `Context`, `PersistenceError`, `DbAttributes`, `with_db_span`, and `PersistenceDriver` are the public abstractions used by services.
- `InMemoryDriver` is a fully-featured in-memory implementation used extensively by tests and stress harnesses.
- `SqliteDriver` is a lightweight PoC driver intended for quick integration tests; it is feature-gated behind the `persistence` Cargo feature.

Feature gating
- The `SqliteDriver` and `sqlx` support are only compiled when the `persistence` feature is enabled for the `commons` crate.
- The `commons` crate's `Cargo.toml` declares `persistence = ["sqlx"]` so downstream crates can enable SQL drivers with `--features persistence`.

Running tests
- Run the normal commons test suite:

```powershell
cd C:\Users\Jon\Documents\Project-Asimov\robotorq-reserve-system
cargo test -p commons
```

- Run the sqlite PoC integration test (feature-gated):

```powershell
cargo test -p commons --features persistence --test persistence_sqlite_tests -- --nocapture
```

Notes & next steps
- SQLite in-memory is suitable for functional verification but does not match Postgres semantics exactly. Use it for PoC and CI smoke tests.
- Recommended next expansions:
  - Implement `ping` (done) and transaction helpers (next)
  - Add a migration runner (`sqlx::migrate!`) and directory-based migrations for production readiness
  - Add a Postgres driver with `PgPool` and connection pooling for production workloads
