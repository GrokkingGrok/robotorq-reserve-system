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
    - SQLite bootstrap: the driver ensures a core `kv(key TEXT PRIMARY KEY, value TEXT)` table exists at init.
    - Schema version: if a `schema_version(version INTEGER)` table exists, its single row must match the expected version.
      - The helper migration `sql/sqlite_migrations/0001_create_schema_version.sql` creates and sets the version to `1`.
      - To repair mismatches, update the row: `UPDATE schema_version SET version = 1;` and re-run the service.
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
  - SQLite version check: when `schema_version` is present, the driver will error on startup if the version mismatches. This flips readiness to not ready.
 - Postgres: uses `information_schema.tables` scoped to the current schema to ensure core tables and the migrations table exist.
 - The Postgres driver caches validation at startup and exposes `health()` for readiness wiring.

Notes & next steps
 - SQLite in-memory is suitable for functional verification but does not match Postgres semantics exactly. Use it for PoC and CI smoke tests.
 - Obfuscation policy: use `commons::util::persistence::obfuscate_statement` to generate stable identifiers for SQL statements in spans and logs (avoids leaking literals).
 - Migrations: both SQLite and Postgres drivers support directory-based migrations under `crates/commons/sql/migrations`, tracking progress in `PersistenceConfig::migration_table`.
 - Readiness & schema validation: `PostgresDriver::from_config` performs a lightweight `validate_schema` using the configured migration table; the result is cached and surfaced via `health()` for wiring into readiness handlers.
 - Migration runner results: both drivers expose structured outcomes via `MigrationOutcome` (fields: `applied`, `skipped`, `dirty`). Use these in operator tooling and migration reports to assert idempotence and detect dirty states caused by checksum mismatches.
  - Idempotence: running the migration runner multiple times is idempotent — migrations already recorded with matching checksums are reported in `skipped` and not re-applied. If a migration file's checksum changes after being recorded, the runner marks a `dirty` state and fails fast.
 - Integration tests:
   - Postgres ping: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_integration -- --nocapture`
   - Postgres migrations: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_migrate_integration -- --nocapture`
   - Readyz + Postgres: `cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_postgres_integration -- --nocapture`
    - Testcontainers env guard: set `RTQ_ENABLE_TESTCONTAINERS=1` to run container-based tests; otherwise they skip fast.
      - Example:
        - Skip: `$env:RTQ_ENABLE_TESTCONTAINERS="0"; cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_config_verification`
        - Run: `$env:RTQ_ENABLE_TESTCONTAINERS="1"; cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test persistence_postgres_config_verification`
 - Example: wiring readiness with persistence

See also
- `docs/PERSISTENCE_SQLITE.md` — SQLite defaults, readiness semantics, and schema version repair.

## Readiness Semantics

- Flag-first: `/readyz` returns 503 when the service readiness flag is false, regardless of dependencies.
- Driver health: When the flag is true and a `PersistenceDriver` is provided, `/readyz` consults `driver.health()`. If `ready=false`, it returns 503 with the driver’s message.
- Schema validation: `PostgresDriver::from_config` may run a lightweight `validate_schema(...)` at startup. If migrations are disabled or validation fails, `driver.health()` reports `ready=false` with a message like "schema not validated".
- Recommended: enable migrations only in controlled environments; for production, validate schema version at startup and gate readiness accordingly.

### Quick Test Commands

```powershell
cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_postgres_integration
cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_schema_mismatch_integration
cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_schema_version_mismatch
cargo test -p commons --features "persistence-postgres persistence-testcontainers" --test readyz_toggle_flag_integration
```

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
