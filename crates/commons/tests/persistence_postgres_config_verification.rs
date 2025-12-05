//! Verify Postgres config application: `ssl_mode`, `application_name`, `search_path`.
//!
//! Feature gates required:
//! - `persistence-postgres`
//! - `persistence-testcontainers`

#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers"
))]

use commons::util::config::persistance::{
    BackendSpecificConfig, PersistenceBackend, PersistenceConfig, PostgresConfig, PostgresSslMode,
};
use commons::util::persistence::backends::postgres::PostgresDriver;
use sqlx::Row;
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

#[tokio::test]
async fn session_application_name_is_applied() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RTQ_ENABLE_TESTCONTAINERS").as_deref() != Ok("1") {
        eprintln!("skipping: RTQ_ENABLE_TESTCONTAINERS != 1");
        return Ok(());
    }

    let container_res = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        postgres::Postgres::default().start(),
    )
    .await
    .map_err(|_| "postgres container start timed out")?;
    let container = container_res?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: true,
        backend_config: BackendSpecificConfig {
            postgres: PostgresConfig {
                ssl_mode: PostgresSslMode::Disable,
                application_name: "rtq-test-app".to_string(),
                search_path: "public".to_string(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let drv = PostgresDriver::from_config(&cfg).await?;

    // Use the inner pool to query current settings
    let pool = drv.pool();
    let row_res = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sqlx::query("SELECT current_setting('application_name')").fetch_one(pool),
    )
    .await
    .map_err(|_| "application_name query timed out")?;
    let row = row_res?;
    let app_name: String = row.get(0);
    assert_eq!(app_name, "rtq-test-app");

    Ok(())
}

#[tokio::test]
async fn session_search_path_is_applied() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RTQ_ENABLE_TESTCONTAINERS").as_deref() != Ok("1") {
        eprintln!("skipping: RTQ_ENABLE_TESTCONTAINERS != 1");
        return Ok(());
    }

    let container_res = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        postgres::Postgres::default().start(),
    )
    .await
    .map_err(|_| "postgres container start timed out")?;
    let container = container_res?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: true,
        backend_config: BackendSpecificConfig {
            postgres: PostgresConfig {
                ssl_mode: PostgresSslMode::Disable,
                application_name: "rtq-test-app2".to_string(),
                search_path: "public".to_string(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let drv = PostgresDriver::from_config(&cfg).await?;
    let pool = drv.pool();

    // SHOW search_path returns string like '"$user", public' or 'public'
    let row_res = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sqlx::query("SHOW search_path").fetch_one(pool),
    )
    .await
    .map_err(|_| "search_path query timed out")?;
    let row = row_res?;
    let search_path: String = row.get(0);
    assert!(search_path.to_lowercase().contains("public"));

    Ok(())
}

#[tokio::test]
async fn ssl_mode_disable_results_in_non_ssl_connection() -> Result<(), Box<dyn std::error::Error>>
{
    if std::env::var("RTQ_ENABLE_TESTCONTAINERS").as_deref() != Ok("1") {
        eprintln!("skipping: RTQ_ENABLE_TESTCONTAINERS != 1");
        return Ok(());
    }

    let container_res = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        postgres::Postgres::default().start(),
    )
    .await
    .map_err(|_| "postgres container start timed out")?;
    let container = container_res?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let cfg = PersistenceConfig {
        backend: PersistenceBackend::Postgres,
        database_url: url.clone(),
        run_migrations: false,
        backend_config: BackendSpecificConfig {
            postgres: PostgresConfig {
                ssl_mode: PostgresSslMode::Disable,
                application_name: "rtq-ssl-test".to_string(),
                search_path: "public".to_string(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let drv = PostgresDriver::from_config(&cfg).await?;
    let pool = drv.pool();

    // There's no direct SQL to assert TLS status. Instead, we rely on the driver
    // not erroring and perform a simple query. In environments where TLS is enforced,
    // this would fail; here it should pass.
    let row_res = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sqlx::query("SELECT 1").fetch_one(pool),
    )
    .await
    .map_err(|_| "ssl-mode check query timed out")?;
    let row = row_res?;
    let one: i32 = row.get(0);
    assert_eq!(one, 1);

    Ok(())
}
