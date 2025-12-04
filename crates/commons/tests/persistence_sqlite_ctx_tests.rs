#![cfg(feature = "persistence")]

use commons::util::persistence::Context;
use commons::util::persistence::sqlite::SqliteDriver;

#[tokio::test]
async fn sqlite_ctx_timeout_and_success() {
    // Use in-memory DB; migrations ensure kv table exists
    let drv = SqliteDriver::new("sqlite::memory:").await.expect("driver");

    // Deadline zero -> Expect DeadlineExceeded on ctx ops
    let ctx_timeout = Context::with_deadline_from_now(std::time::Duration::from_millis(0));
    let res = drv.put_ctx(&ctx_timeout, "k", "v").await;
    assert!(matches!(
        res,
        Err(commons::util::persistence::PersistenceError::DeadlineExceeded)
    ));

    // Reasonable deadline -> operations succeed
    let ctx_ok = Context::with_deadline_from_now(std::time::Duration::from_millis(100));
    drv.put_ctx(&ctx_ok, "k", "v").await.expect("put_ctx");
    let got = drv.get_ctx(&ctx_ok, "k").await.expect("get_ctx");
    assert_eq!(got, Some("v".to_string()));

    drv.delete_ctx(&ctx_ok, "k").await.expect("delete_ctx");
    let none = drv.get_ctx(&ctx_ok, "k").await.expect("get_ctx2");
    assert_eq!(none, None);
}
