Persistence module (commons)

This folder contains the persistence abstractions and small proof-of-concept drivers for SQLite and Postgres.

Overview
- `Context`, `PersistenceError`, `DbAttributes`, `with_db_span`, and `PersistenceDriver` are the public abstractions used by services.
- `InMemoryDriver` is a fully-featured in-memory implementation used extensively by tests and stress harnesses.
- `SqliteDriver` is a lightweight PoC driver intended for quick integration tests; it is feature-gated behind the `persistence` Cargo feature.
- `PostgresDriver` provides pooled connectivity, session configuration, and directory-based migrations; it is feature-gated behind `persistence-postgres`.

Feature gating
- The `SqliteDriver` and `sqlx` support are compiled when the `persistence` feature is enabled for the `commons` crate.
- The `PostgresDriver` is compiled when the `persistence-postgres` feature is enabled; `testcontainers` integrations require `persistence-testcontainers`.
- The `commons` crate's `Cargo.toml` declares these features so downstream crates can enable SQL drivers with `--features persistence` or `--features persistence-postgres`.

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

Backend selection, readiness, and schema validation
 - Default backend: SQLite
   - The system defaults to SQLite for persistence and readiness checks.
   - Configure via `robotorq.toml`:
     - `persistence.backend = "sqlite"`
     - `persistence.sqlite_url = "sqlite://robotorq.db"`
   - Migrations are tracked in `persistence.migration_table` (default `_robotorq_migrations`).
 - Toggle to Postgres
   - Set `persistence.backend = "postgres"` and provide `DATABASE_URL` (or `postgres_url` in config).
   - Example: `postgres://user:pass@127.0.0.1:5432/dbname`.
   - Enable features: `persistence-postgres` and optionally `persistence-testcontainers` for integration tests.

  Postgres options (defaults and opt-ins)
  - `ssl_mode` (default: `prefer`): one of `require|prefer|allow|disable`; appended to URL as `sslmode=`.
  - `application_name` (default: `robotorq`): appended to URL and reinforced via `set_config('application_name', ...)` after connect.
  - `search_path` (default: `public`): applied via `set_config('search_path', ...)` after connect.
  - Pool: honors `max_connections`, `idle_timeout_seconds`, `max_lifetime_seconds`.
  - Connect timeout: `connect_timeout_seconds` wraps initial connect in a timeout.

Readiness & schema validation
 - SQLite: checks presence of core tables and migrations tracking table.
 - Postgres: uses `information_schema.tables` scoped to the current schema to ensure core tables and the migrations table exist.
 - The Postgres driver caches validation at startup and exposes `health()` for readiness wiring.

Notes & next steps
 - SQLite in-memory is suitable for functional verification but does not match Postgres semantics exactly. Use it for PoC and CI smoke tests.
 - Obfuscation policy: use `commons::util::persistence::obfuscate_statement` to generate stable identifiers for SQL statements in spans and logs (avoids leaking literals).
 - Migrations: both SQLite and Postgres drivers support directory-based migrations under `crates/commons/sql/migrations`, tracking progress in `PersistenceConfig::migration_table`.
 - Readiness & schema validation: `PostgresDriver::from_config` performs a lightweight `validate_schema` using the configured migration table; the result is cached and surfaced via `health()` for wiring into readiness handlers.
 - Integration tests:
   - Postgres ping: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_integration -- --nocapture`
   - Postgres migrations: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_migrate_integration -- --nocapture`
   - Readyz + Postgres: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_postgres_integration -- --nocapture`
 - Example: wiring readiness with persistence

```rust
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use commons::services::robotorq_service::readyz::readyz_handler_with_driver;
use commons::util::config::persistance::{PersistenceConfig, PersistenceBackend};
use commons::util::persistence::backends::postgres::PostgresDriver;
use commons::util::persistence::PersistenceDriver;

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let mut cfg = PersistenceConfig::default();
cfg.backend = PersistenceBackend::Postgres;
cfg.database_url = "postgres://user:pass@localhost:5432/db".to_string();
cfg.run_migrations = true;
let drv = PostgresDriver::from_config(&cfg).await?;
let ready_flag = Arc::new(AtomicBool::new(true));
let resp = readyz_handler_with_driver(ready_flag, Some(Arc::new(drv) as Arc<dyn PersistenceDriver>));
let (parts, _) = axum::response::IntoResponse::into_response(resp).into_parts();
assert_eq!(parts.status, axum::http::StatusCode::OK);
# Ok(()) }
```

CLI usage
 - SQLite:
   - `cargo run -p cargo-robotorq-migrate -- sqlite sqlite://robotorq.db _robotorq_migrations`
 - Postgres:
   - `cargo run -p cargo-robotorq-migrate -- postgres $env:DATABASE_URL _robotorq_migrations`
 - Recommended next expansions:
   - Postgres health and schema validation APIs
   - Statement obfuscation usage across all driver operations
   - Transaction helpers for Postgres mirroring SQLite patterns
