#![cfg(feature = "persistence")]

use commons::util::persistence::SqliteDriver;

#[tokio::test]
async fn sqlite_basic_kv_roundtrip() {
    // Use an in-memory SQLite database for quick tests.
    let drv = SqliteDriver::new("sqlite::memory:")
        .await
        .expect("create driver");

    drv.put("k1", "v1").await.expect("put");
    let v = drv.get("k1").await.expect("get");
    assert_eq!(v, Some("v1".to_string()));

    drv.put("k1", "v2").await.expect("put2");
    let v2 = drv.get("k1").await.expect("get2");
    assert_eq!(v2, Some("v2".to_string()));

    drv.delete("k1").await.expect("delete");
    let v3 = drv.get("k1").await.expect("get3");
    assert_eq!(v3, None);
}

#[tokio::test]
async fn sqlite_migration_table_and_pragmas_applied() {
    use commons::util::config::persistance::{
        PersistenceBackend, PersistenceConfig, SqliteConfig, SqliteJournalMode,
        SqliteSynchronousMode,
    };
    use sqlx::Row;

    // Use an in-memory SQLite DB; we can inspect schema via the same connection.
    let db_url = "sqlite::memory:";

    // Build config with custom migration table and PRAGMAs.
    let mut cfg = PersistenceConfig::default();
    cfg.backend = PersistenceBackend::Sqlite;
    cfg.database_url = db_url.to_string();
    cfg.run_migrations = true;
    cfg.migration_table = "_rtq_migrations_test".to_string();
    cfg.backend_config.sqlite = SqliteConfig {
        foreign_keys: true,
        journal_mode: SqliteJournalMode::Wal,
        synchronous: SqliteSynchronousMode::Normal,
        cache_size_kb: -1024,
        busy_timeout_ms: 7000,
    };

    let drv = SqliteDriver::from_config(&cfg)
        .await
        .expect("driver from config");

    // Verify migration tracking table exists.
    let mut conn = drv.acquire_connection().await.expect("conn");
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name = ?;")
            .bind(&cfg.migration_table)
            .fetch_optional(&mut *conn)
            .await
            .expect("query sqlite_master");
    assert!(exists.is_some(), "migration table not created");

    // Verify foreign_keys PRAGMA is ON.
    let fk_row = sqlx::query("PRAGMA foreign_keys;")
        .fetch_one(&mut *conn)
        .await
        .expect("pragma foreign_keys");
    let fk_on: i32 = fk_row.get(0);
    assert_eq!(fk_on, 1);
}
