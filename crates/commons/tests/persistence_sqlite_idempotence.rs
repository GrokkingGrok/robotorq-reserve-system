#![cfg(feature = "persistence")]

use sqlx::sqlite::SqlitePool;

use commons::util::persistence::sqlite::run_migrations_structured;

/// Verify migrations are idempotent: running the runner twice applies on first run
/// and reports skipped on the second run (same filenames/checksums).
#[tokio::test]
async fn sqlite_migrations_idempotence() -> Result<(), Box<dyn std::error::Error>> {
    // Use an in-memory SQLite instance for fast tests.
    let pool = SqlitePool::connect("sqlite::memory:").await?;

    // Use a unique migrations table for the test to avoid clashing with other tests.
    let table_name = "_robotorq_migrations_idempotence_test";

    // First run: expect some migrations to be applied (or none if no migrations present).
    let first = run_migrations_structured(&pool, table_name).await?;

    // Second run: expect no applied migrations, and previously applied ones reported as skipped.
    let second = run_migrations_structured(&pool, table_name).await?;

    // If first run applied migrations, the second run should skip exactly those.
    if first.applied.is_empty() {
        // If there were no migrations to apply on first run, second should also yield no applied and no skipped.
        assert!(second.applied.is_empty(), "no applied on second run");
        assert!(second.skipped.is_empty(), "no skipped on second run");
    } else {
        assert!(
            second.applied.is_empty(),
            "second run should not apply migrations"
        );
        assert_eq!(
            second.skipped, first.applied,
            "second run skipped set should equal first applied set"
        );
    }

    Ok(())
}
