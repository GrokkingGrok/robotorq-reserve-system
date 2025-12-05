//! Readyz integration: toggling the readiness flag overrides persistence health.
//!
//! Requires features:
//! - `persistence-postgres`
//! - `persistence-testcontainers`

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use commons::services::robotorq_service::readyz::readyz_handler_with_driver;
use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::PersistenceDriver;
use commons::util::persistence::backends::postgres::PostgresDriver;

#[tokio::test]
async fn readyz_flips_with_flag_even_if_persistence_ready() -> Result<(), Box<dyn std::error::Error>>
{
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

    let flag = Arc::new(AtomicBool::new(true));
    let response_ok = readyz_handler_with_driver(
        flag.clone(),
        Some(Arc::new(drv) as Arc<dyn PersistenceDriver>),
    );
    let (parts_ok, body_ok) = axum::response::IntoResponse::into_response(response_ok).into_parts();
    assert_eq!(parts_ok.status, axum::http::StatusCode::OK);
    let body_bytes_ok = axum::body::to_bytes(body_ok, usize::MAX).await?;
    assert_eq!(&body_bytes_ok[..], b"Ready");

    // Flip flag to false and assert 503 regardless of driver health
    flag.store(false, Ordering::Relaxed);
    let response_bad = readyz_handler_with_driver(flag.clone(), None);
    let (parts_bad, body_bad) =
        axum::response::IntoResponse::into_response(response_bad).into_parts();
    assert_eq!(
        parts_bad.status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body_bytes_bad = axum::body::to_bytes(body_bad, usize::MAX).await?;
    assert_eq!(&body_bytes_bad[..], b"Not Ready");

    Ok(())
}
