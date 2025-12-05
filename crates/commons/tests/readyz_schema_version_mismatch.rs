//! Readyz integration: readiness returns 503 on schema version mismatch.
//!
//! Requires features:
//! - `persistence-postgres`
//! - `persistence-testcontainers`

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use std::sync::{Arc, atomic::AtomicBool};

use commons::services::robotorq_service::readyz::readyz_handler_with_driver;
use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::PersistenceDriver;
use commons::util::persistence::backends::postgres::PostgresDriver;
use commons::util::schema::current_schema_version;

/// When the database lacks the expected schema version marker, readiness should be 503.
#[tokio::test]
async fn readyz_reports_503_on_schema_version_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    use testcontainers_modules::postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url,
        run_migrations: false, // avoid creating migration table; driver should report not validated
        ..Default::default()
    };

    let drv = PostgresDriver::from_config(&cfg).await?;

    // Ensure we have an expected version for a type
    let _expected = current_schema_version("Token").expect("known schema type");

    // Simulate mismatch: drop or avoid recording version marker table/row.
    // For now, we simply call validate_schema on a wrong table name to force failure.
    let ctx = commons::util::persistence::ctx_with_timeout_ms(2000);
    // Optional: attempt validation with wrong table (should fail), but health uses cached flag
    let _ = drv
        .validate_schema(&ctx, "_robotorq_migrations_wrong")
        .await;

    // Ready flag true, but persistence not validated => 503
    let flag = Arc::new(AtomicBool::new(true));
    let response =
        readyz_handler_with_driver(flag, Some(Arc::new(drv) as Arc<dyn PersistenceDriver>));
    let (parts, body) = axum::response::IntoResponse::into_response(response).into_parts();
    assert_eq!(parts.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();
    assert!(
        body_str.to_lowercase().contains("schema") && body_str.to_lowercase().contains("validated")
    );

    Ok(())
}
