#![cfg(feature = "persistence")]

use commons::util::config::persistance::PersistenceConfig;
use commons::util::persistence::{ctx_with_timeout_ms, make_driver};

#[tokio::test]
async fn driver_factory_sqlite_smoke() {
    let cfg = PersistenceConfig {
        backend: commons::util::config::persistance::PersistenceBackend::Sqlite,
        database_url: "sqlite://:memory:".to_string(),
        run_migrations: false, // keep it fast for tests
        ..Default::default()
    };

    let drv = make_driver(&cfg).await.expect("make sqlite driver");

    let health = drv.health();
    assert!(health.ready);

    let ctx = ctx_with_timeout_ms(1000);
    // trait ping is synchronous default; in-memory and sqlite drivers should respond
    let res = drv.ping(&ctx).await;
    assert!(res.is_ok());
}
