//! Integration test: `schema_version` table value mismatch should flip readiness.
//!
//! Requires features: persistence-postgres, persistence-testcontainers.

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use commons::util::config::persistance::{
    PersistenceBackend, PersistenceConfig, PostgresConfig, PostgresSslMode,
};
use commons::util::persistence::backends::postgres::PostgresDriver;
use commons::util::persistence::{Context, PersistenceDriver};
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

#[tokio::test]
async fn readiness_flips_on_schema_version_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RTQ_ENABLE_TESTCONTAINERS").as_deref() != Ok("1") {
        eprintln!("skipping: RTQ_ENABLE_TESTCONTAINERS != 1");
        return Ok(());
    }

    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    // Build config with migrations enabled so our 0001_create_schema_version.sql applies
    let mut cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: true,
        ..Default::default()
    };
    cfg.backend_config.postgres = PostgresConfig {
        ssl_mode: PostgresSslMode::Disable,
        application_name: "rtq-readyz-test".to_string(),
        search_path: "public".to_string(),
    };

    let drv = PostgresDriver::from_config(&cfg).await?;

    // First, health should be ready because schema_version = 1 matches expected constant
    let h1 = drv.health();
    assert!(
        h1.ready,
        "expected ready after migrations: {:?}",
        h1.message
    );

    // Flip schema_version to a wrong value (e.g., 99) and re-validate
    let pool = drv.pool();
    sqlx::query("UPDATE schema_version SET version = 99")
        .execute(pool)
        .await?;

    // Validate again via API; should return an error now
    let ctx = Context::default();
    let res = drv.validate_schema(&ctx, &cfg.migration_table).await;
    assert!(
        res.is_err(),
        "schema validation should fail on version mismatch"
    );

    Ok(())
}
