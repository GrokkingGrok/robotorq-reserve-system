//! Readyz integration: readiness flips to 503 on schema mismatch.
//!
//! Feature gates required:
//! - `persistence-postgres`
//! - `persistence-testcontainers`

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use std::sync::{Arc, atomic::AtomicBool};

use commons::services::robotorq_service::readyz::readyz_handler_with_driver;
use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::backends::postgres::PostgresDriver;
use commons::util::persistence::{Context, PersistenceDriver};

/// If the expected migration table is missing, validate_schema should fail and readiness returns 503.
#[tokio::test]
async fn readyz_reports_503_on_migration_table_missing() -> Result<(), Box<dyn std::error::Error>> {
    use testcontainers_modules::postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let mut cfg = PersistenceConfig::default();
    cfg.backend = PersistenceBackend::Postgres;
    cfg.database_url = url;
    cfg.run_migrations = false; // Do not create migration table

    let drv = PostgresDriver::from_config(&cfg).await?;

    // Explicitly validate a non-existent migration table name
    let ctx = commons::util::persistence::ctx_with_timeout_ms(2000);
    let bad = drv
        .validate_schema(&ctx, "_robotorq_migrations_missing")
        .await;
    assert!(
        bad.is_err(),
        "schema validation should fail for missing migration table"
    );

    // Even if service flag is true, readiness should report 503 when driver health fails
    let flag = Arc::new(AtomicBool::new(true));
    let response =
        readyz_handler_with_driver(flag, Some(Arc::new(drv) as Arc<dyn PersistenceDriver>));
    let (parts, body) = axum::response::IntoResponse::into_response(response).into_parts();
    assert_eq!(parts.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();
    assert!(
        body_str.to_lowercase().contains("schema") && body_str.to_lowercase().contains("validated"),
        "unexpected body: {body_str}"
    );

    Ok(())
}
