//! Readyz + Postgres integration tests (feature-gated).
//!
//! Uses `testcontainers` to spin up Postgres, runs migrations via the
//! `PostgresDriver`, and asserts the HTTP readiness handler reports 200 OK
//! when both the service flag and persistence schema are ready.
//!
//! Feature gates required:
//! - `persistence-postgres`
//! - `persistence-testcontainers`
//!
//! Note: These tests are integration-level and may take several seconds.

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use std::sync::{Arc, atomic::AtomicBool};

use commons::services::robotorq_service::readyz::readyz_handler_with_driver;
use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::PersistenceDriver;
use commons::util::persistence::backends::postgres::PostgresDriver;
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

/// Start a Postgres container, construct a driver with migrations enabled,
/// and verify the readyz handler returns 200 OK once schema is validated.
#[tokio::test]
async fn readyz_reports_ok_with_valid_schema() -> Result<(), Box<dyn std::error::Error>> {
    // Start a Postgres container (defaults are fine for sqlx)
    let container = postgres::Postgres::default().start().await?;

    // Build a connection URL from the container
    let port = container.get_host_port_ipv4(5432).await?;
    let user = "postgres";
    let pass = "postgres";
    let db = "postgres";
    let url = format!("postgres://{user}:{pass}@127.0.0.1:{port}/{db}");

    // Configure persistence to use Postgres with migrations enabled
    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: true,
        ..Default::default()
    };

    // Construct driver (runs migrations + schema validation)
    let drv = PostgresDriver::from_config(&cfg).await?;

    // Service readiness flag true
    let flag = Arc::new(AtomicBool::new(true));
    let response =
        readyz_handler_with_driver(flag, Some(Arc::new(drv) as Arc<dyn PersistenceDriver>));

    // Assert HTTP 200 OK
    let (parts, body) = axum::response::IntoResponse::into_response(response).into_parts();
    assert_eq!(parts.status, axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
    assert_eq!(&body_bytes[..], b"Ready");

    Ok(())
}

/// Validate that when the service flag is false, readyz returns 503 even
/// if persistence is healthy.
#[tokio::test]
async fn readyz_reports_503_when_flag_false() -> Result<(), Box<dyn std::error::Error>> {
    use testcontainers_modules::postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let mut cfg = PersistenceConfig::default();
    cfg.backend = PersistenceBackend::Postgres;
    cfg.database_url = url;
    cfg.run_migrations = true;

    let drv = PostgresDriver::from_config(&cfg).await?;

    let flag = Arc::new(AtomicBool::new(false));
    let response =
        readyz_handler_with_driver(flag, Some(Arc::new(drv) as Arc<dyn PersistenceDriver>));

    let (parts, body) = axum::response::IntoResponse::into_response(response).into_parts();
    assert_eq!(parts.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
    assert_eq!(&body_bytes[..], b"Not Ready");

    Ok(())
}
