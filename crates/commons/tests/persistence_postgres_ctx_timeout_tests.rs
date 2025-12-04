//! Postgres CRUD timeout tests using Context deadlines (feature-gated).
//!
//! Requires features:
//! - `persistence-postgres`
//! - `persistence-testcontainers`

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use std::time::Duration;

use commons::util::config::persistance::{PersistenceBackend, PersistenceConfig};
use commons::util::persistence::backends::postgres::PostgresDriver;
use commons::util::persistence::{Context, PersistenceError};

/// When the Context deadline has already expired, CRUD ops should return Timeout.
#[tokio::test]
async fn put_ctx_returns_timeout_on_expired_deadline() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create a Context with a deadline already expired
    let ctx_timeout = Context {
        trace_headers: Default::default(),
        deadline: Some(tokio::time::Instant::now() + Duration::from_millis(1)),
        metadata: None,
    };
    // Ensure the deadline is past
    tokio::time::sleep(Duration::from_millis(10)).await;

    let res = drv.put_ctx(&ctx_timeout, "k_timeout", "v").await;
    match res {
        Err(PersistenceError::DeadlineExceeded) => {}
        other => panic!("expected DeadlineExceeded, got: {:?}", other),
    }

    Ok(())
}

/// Sanity check: with a reasonable deadline, CRUD should succeed.
#[tokio::test]
async fn put_get_delete_ctx_succeeds_with_reasonable_deadline()
-> Result<(), Box<dyn std::error::Error>> {
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

    let ctx_ok = Context {
        trace_headers: Default::default(),
        deadline: Some(tokio::time::Instant::now() + Duration::from_secs(2)),
        metadata: None,
    };

    drv.put_ctx(&ctx_ok, "k_ok", "v").await.expect("put_ctx");
    let got = drv.get_ctx(&ctx_ok, "k_ok").await.expect("get_ctx");
    assert_eq!(got, Some("v".to_string()));
    drv.delete_ctx(&ctx_ok, "k_ok").await.expect("delete_ctx");

    Ok(())
}
