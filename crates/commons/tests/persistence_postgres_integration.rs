#![cfg(all(
    feature = "persistence-postgres",
    feature = "persistence-testcontainers",
    not(miri)
))]

use commons::util::config::persistance::PersistenceConfig;
use commons::util::persistence::make_driver;

// Use the helper crate that provides typed modules for testcontainers
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

// This integration test uses Docker via testcontainers-modules to start a
// Postgres container, then constructs a `PersistenceConfig` pointing at the
// container and asserts that the PostgresDriver::ping() call succeeds.
#[tokio::test]
async fn postgres_driver_ping_integration() {
    // Start the Postgres image via the postgres module (uses the crate's async
    // runner under the hood).
    let container = postgres::Postgres::default()
        .start()
        .await
        .expect("start postgres");

    // Get the host port for forwarded 5432 and build the connection URL.
    let host_port = container.get_host_port_ipv4(5432).await.expect("host port");
    let url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        host_port
    );

    // Build a PersistenceConfig that points to the container
    let mut cfg = PersistenceConfig::default();
    cfg.backend = commons::util::config::persistance::PersistenceBackend::Postgres;
    cfg.database_url = url;
    cfg.max_connections = 2;
    cfg.min_connections = 1;
    cfg.connect_timeout_seconds = 10;
    cfg.run_migrations = false; // keep this test focused on connectivity

    // Attempt to create a driver via the factory. The factory will construct
    // a PostgresDriver and attempt to connect using sqlx.
    let drv = make_driver(&cfg).await.expect("make postgres driver");

    // Use a 5s deadline to ensure ping doesn't hang on failure
    let ctx = commons::util::persistence::ctx_with_timeout_ms(5000);
    let res = drv.ping(&ctx).await;
    assert!(res.is_ok(), "Postgres ping failed: {:?}", res);

    // Shutdown/cleanup if driver supports it
    drv.shutdown();
}
