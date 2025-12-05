#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers",
    not(miri)
))]

use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::make_driver;
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

/// Starts a Postgres container, runs migrations via `make_driver` with
/// `run_migrations = true`, then verifies both the `kv` table and the
/// migration tracking row exist.
#[tokio::test]
async fn postgres_driver_runs_migrations_integration() {
    let container = postgres::Postgres::default()
        .start()
        .await
        .expect("start postgres");

    let host_port = container.get_host_port_ipv4(5432).await.expect("host port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        max_connections: 2,
        min_connections: 1,
        connect_timeout_seconds: 10,
        run_migrations: true,
        ..Default::default()
    };
    // Use default migration table name
    let mig_table = cfg.migration_table.clone();

    let drv = make_driver(&cfg).await.expect("make postgres driver");

    // Verify kv table exists by running a simple query using sqlx directly.
    let pool = sqlx::PgPool::connect(&url).await.expect("connect pool");
    let _ = sqlx::query("SELECT 1 FROM kv LIMIT 1")
        .fetch_optional(&pool)
        .await
        .expect("query kv");

    // Verify a migration row exists for 0001_create_kv.sql
    let row: Option<(i64,)> = sqlx::query_as(&format!(
        "SELECT applied_at FROM {mig_table} WHERE filename = $1"
    ))
    .bind("0001_create_kv.sql")
    .fetch_optional(&pool)
    .await
    .expect("migration row");

    assert!(
        row.is_some(),
        "expected migration row for 0001_create_kv.sql"
    );

    drv.shutdown();
}
