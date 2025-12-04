use commons::util::config::persistance::PersistenceConfig;
use commons::util::persistence::{ctx_with_timeout_ms, make_driver};

#[tokio::test]
async fn driver_factory_memory_smoke() {
    let mut cfg = PersistenceConfig::default();
    cfg.backend = commons::util::config::persistance::PersistenceBackend::Memory;

    let drv = make_driver(&cfg).await.expect("make driver");

    let health = drv.health();
    assert!(health.ready);

    // ping should succeed with non-expired context
    let ctx = ctx_with_timeout_ms(1000);
    let res = drv.ping(&ctx).await;
    assert!(res.is_ok());
}
