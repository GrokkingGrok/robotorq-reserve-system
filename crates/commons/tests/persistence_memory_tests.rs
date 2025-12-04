use commons::util::persistence::{Context, InMemoryDriver, PersistenceDriver, PersistenceError};
use std::time::Duration;

#[test]
fn ping_ok_when_not_expired() {
    let drv = InMemoryDriver::new();
    let ctx = Context::with_deadline_from_now(Duration::from_millis(100));
    // async ping on the trait; run via tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async { drv.ping(&ctx).await });
    assert!(res.is_ok());
}

#[tokio::test]
async fn ping_deadline_exceeded_when_expired() {
    let drv = InMemoryDriver::new();
    let ctx = Context::with_deadline_from_now(Duration::from_millis(10));
    // wait for expiry
    tokio::time::sleep(Duration::from_millis(20)).await;
    let res = drv.ping(&ctx).await;
    assert_eq!(res, Err(PersistenceError::DeadlineExceeded));
}
