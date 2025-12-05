//! `SQLite` readiness behavior: `schema_version` mismatch should cause startup error or not-ready.

use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::sqlite::SqliteDriver;
use sqlx::sqlite::SqlitePool;

#[tokio::test]
async fn sqlite_schema_version_mismatch_blocks_driver_init() {
    // Use file-based temp DB to persist across operations during test
    // Use file path that exists; sqlx sqlite connection string format: sqlite://<path>
    // Use in-memory DB to avoid filesystem issues; use shared memory so multiple connections work
    // Use a shared in-memory database name unique to this process so that
    // multiple connections (the test setup and the driver) see the same DB.
    let db_path = format!(
        "sqlite:file:rtq_schema_test_{}?mode=memory&cache=shared",
        std::process::id()
    );

    // Pre-create schema_version with wrong value
    let pool = SqlitePool::connect(&db_path).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS schema_version(version INTEGER NOT NULL);")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM schema_version;")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO schema_version(version) VALUES (99);")
        .execute(&pool)
        .await
        .unwrap();

    // Attempt to initialize driver; should error due to mismatch
    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Sqlite,
        database_url: db_path.clone(),
        run_migrations: false, // we're controlling schema_version manually
        ..Default::default()
    };

    let res = SqliteDriver::from_config(&cfg).await;
    assert!(
        res.is_err(),
        "expected init error on schema version mismatch"
    );
}
