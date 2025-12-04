use commons::util::persistence::{
    PersistenceDriver, PersistenceError, ctx_with_timeout_ms, expired_ctx, memory_driver,
};

#[test]
fn helper_memory_driver_ping_ok() {
    let drv = memory_driver();
    let ctx = ctx_with_timeout_ms(100);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async { drv.ping(&ctx).await });
    assert!(res.is_ok());
}

#[test]
fn helper_expired_ctx_triggers_deadline() {
    let drv = memory_driver();
    let ctx = expired_ctx();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async { drv.ping(&ctx).await });
    assert_eq!(res, Err(PersistenceError::DeadlineExceeded));
}
