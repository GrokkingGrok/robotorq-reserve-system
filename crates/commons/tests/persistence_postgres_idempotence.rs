#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::backends::postgres::PostgresDriver;
use std::collections::HashSet;
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

/// Postgres idempotence test: run migrations twice and assert the second run
/// does not re-apply already-applied migrations. Guarded by `RTQ_ENABLE_TESTCONTAINERS`.
#[tokio::test]
async fn postgres_migrations_idempotence() -> Result<(), Box<dyn std::error::Error>> {
    // Skip when Testcontainers isn't enabled (e.g., developer machine without Docker)
    if std::env::var("RTQ_ENABLE_TESTCONTAINERS").unwrap_or_else(|_| "0".to_string()) != "1" {
        eprintln!("RTQ_ENABLE_TESTCONTAINERS != 1; skipping Postgres idempotence test");
        return Ok(());
    }

    // Start Postgres container
    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    // Unique migration table for this test
    let migration_table = "_robotorq_migrations_postgres_idempotence";

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: true,
        migration_table: migration_table.to_string(),
        ..Default::default()
    };

    // First construction should run migrations and populate the migration table
    let _drv1 = PostgresDriver::from_config(&cfg).await?;

    // Connect directly with sqlx to inspect migration table contents
    let pool = sqlx::PgPool::connect(&url).await?;
    let query = format!("SELECT filename FROM {migration_table};");
    let rows: Vec<(String,)> = sqlx::query_as(&query).fetch_all(&pool).await?;
    let first_set: HashSet<String> = rows.into_iter().map(|(f,)| f).collect();

    // Second construction should skip applied migrations (idempotent)
    let _drv2 = PostgresDriver::from_config(&cfg).await?;
    let rows2: Vec<(String,)> = sqlx::query_as(&query).fetch_all(&pool).await?;
    let second_set: HashSet<String> = rows2.into_iter().map(|(f,)| f).collect();

    assert_eq!(
        first_set, second_set,
        "migration set should be unchanged after second run"
    );

    Ok(())
}
