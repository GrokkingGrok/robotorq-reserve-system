//! Readyz integration: combined scenarios for flag + persistence health precedence.
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

/// With flag=true but persistence not validated, readiness should be 503; flipping flag=false keeps 503 with "Not Ready".
#[tokio::test]
async fn flag_true_persistence_not_ready_then_flip_flag_false()
-> Result<(), Box<dyn std::error::Error>> {
    use testcontainers_modules::postgres;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = postgres::Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    // Start driver with migrations disabled so schema_validated stays false.
    let mut cfg = PersistenceConfig::default();
    cfg.backend = PersistenceBackend::Postgres;
    cfg.database_url = url;
    cfg.run_migrations = false;

    let drv = PostgresDriver::from_config(&cfg).await?;

    let flag = Arc::new(AtomicBool::new(true));
    let response_bad = readyz_handler_with_driver(
        flag.clone(),
        Some(Arc::new(drv) as Arc<dyn PersistenceDriver>),
    );
    let (parts_bad, body_bad) =
        axum::response::IntoResponse::into_response(response_bad).into_parts();
    assert_eq!(
        parts_bad.status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body_bytes_bad = axum::body::to_bytes(body_bad, usize::MAX).await?;
    let body_str_bad = String::from_utf8(body_bytes_bad.to_vec()).unwrap_or_default();
    assert!(
        body_str_bad.to_lowercase().contains("schema")
            || body_str_bad.to_lowercase().contains("persistence")
    );

    // Flip flag to false; readiness should be 503 "Not Ready" regardless.
    flag.store(false, Ordering::Relaxed);
    let response_flag = readyz_handler_with_driver(flag.clone(), None);
    let (parts_flag, body_flag) =
        axum::response::IntoResponse::into_response(response_flag).into_parts();
    assert_eq!(
        parts_flag.status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    let body_bytes_flag = axum::body::to_bytes(body_flag, usize::MAX).await?;
    assert_eq!(&body_bytes_flag[..], b"Not Ready");

    Ok(())
}
