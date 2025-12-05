# SQLite Persistence — Defaults, Readiness, and Migrations

This guide summarizes how the RoboTorq template uses SQLite by default for development and readiness, and how to manage schema versioning.

## Defaults
- Backend: SQLite is the default persistence backend.
- Driver: `commons::util::persistence::sqlite::SqliteDriver` (feature: `persistence`).
- Core table bootstrap: the driver ensures `kv(key TEXT PRIMARY KEY, value TEXT NOT NULL)` exists at initialization.
- Migrations: directory-based migrations under `crates/commons/sql/migrations` tracked by `PersistenceConfig.migration_table` (default `_robotorq_migrations`).

## Schema Versioning
- Optional table: `schema_version(version INTEGER)` with a single row.
- Driver behavior: if `schema_version` exists, its value must match `commons::util::schema::ROBOTORQ_CONFIG_SCHEMA_VERSION`.
- Startup validation: `SqliteDriver::from_config(...)` checks `kv` and `schema_version` (when present). A mismatch returns an error and flips readiness to not ready in services.

### Repair Steps
- Update the version row to the expected value and restart:
```sql
UPDATE schema_version SET version = 1;
```
- Or re-apply the helper migration:
  - `crates/commons/sql/sqlite_migrations/0001_create_schema_version.sql` creates and sets the version to 1.

## Example (CRUD + Context)
```rust
use commons::util::persistence::sqlite::SqliteDriver;
use commons::util::persistence::Context;
# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let drv = SqliteDriver::new("sqlite::memory:").await?;
let ctx = Context::with_deadline_from_now(std::time::Duration::from_millis(100));
drv.put_ctx(&ctx, "k", "v").await?;
let v = drv.get_ctx(&ctx, "k").await?.unwrap();
assert_eq!(v, "v");
drv.delete_ctx(&ctx, "k").await?;
# Ok(()) }
```

## Tests (SQLite-only)
Run the SQLite feature-gated tests:
```powershell
cargo test -p commons --features "persistence"
```
Readiness mismatch test (schema_version incorrect):
```powershell
cargo test -p commons --test readyz_sqlite_schema_version_mismatch --features "persistence"
```

## Readiness Endpoints

- `GET /readyz`: lightweight, cached readiness. This is the default probe used by the server and is optimized to avoid a DB hit on every probe. Use this for standard Kubernetes readiness checks.
- `GET /readyz-strict`: strict, per-request validation. Calls into the persistence driver's `health_now()` and performs current schema/driver validation. Use this for targeted operational checks (e.g., after applying migrations or during incident triage).

When deploying, prefer `GET /readyz` for liveness/readiness probes to minimize load. Use `GET /readyz-strict` from runbooks or operator scripts when you need an authoritative, up-to-date verification of persistence readiness.

## Postgres (Deferred)
When ready to switch, enable Postgres features and use Testcontainers for integration tests. Until then, keep SQLite as the default for readiness and basic CRUD.
