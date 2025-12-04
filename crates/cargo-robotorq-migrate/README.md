# cargo-robotorq-migrate

A tiny migration runner for the RoboTorq persistence layer.

## What are migrations?

Migrations are versioned changes to your database schema, applied safely and in order. Each migration is a plain SQL file (e.g., `0001_create_kv.sql`) placed under `crates/commons/sql/migrations`. The runner applies these files one by one and records a row per file in a tracking table (default: `_robotorq_migrations`) that includes:

- filename: the SQL file name
- checksum: a hash of the file contents to detect changes
- applied_at: a timestamp indicating when it was applied

This gives you:
- Ordering: files like `0001_...`, `0002_...` run in sequence
- Idempotency: already-applied files are skipped
- Safety: if a file changes after application, the checksum mismatch is detected

## Usage

```
# Run migrations for SQLite
cargo run -p cargo-robotorq-migrate -- sqlite <DATABASE_URL> [TABLE_NAME]

# Examples
cargo run -p cargo-robotorq-migrate -- sqlite sqlite://robotorq.db
cargo run -p cargo-robotorq-migrate -- sqlite sqlite://file:memdb1?mode=memory&_foreign_keys=on _robotorq_migrations

# Help
cargo run -p cargo-robotorq-migrate -- --help
```

Arguments:
- `<DATABASE_URL>`: Connection string like `sqlite://robotorq.db`
- `[TABLE_NAME]`: Optional tracking table name (defaults to `_robotorq_migrations`)

Environment:
- If `<DATABASE_URL>` is omitted, `DATABASE_URL` env var is used.

## How it works

The CLI calls the commons migration helpers which:
- Read all `*.sql` files from `crates/commons/sql/migrations`
- Create the tracking table if missing
- For each file:
  - Compute its checksum
  - Skip if a row with the same filename+checksum exists
  - Otherwise execute the SQL inside a transaction, then record a row

If any SQL fails, the transaction is rolled back and the error is surfaced.

## Tips
- Keep migration files small and atomic. One responsibility per file.
- Use consistent naming like `0001_create_kv.sql`, `0002_add_index.sql`.
- Avoid editing a migration after it has been applied; add a new file instead.
