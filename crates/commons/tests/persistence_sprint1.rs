use commons::util::persistence::{Context, DbAttributes, PersistenceError, with_db_span};

#[tokio::test]
async fn context_deadline_and_with_db_span_timeout() {
    // No deadline -> operation completes
    let ctx = Context::new();
    let attrs = DbAttributes {
        driver: Some("memory".to_string()),
        ..Default::default()
    };

    let res = with_db_span(&ctx, &attrs, async { Ok::<_, PersistenceError>(42u32) }).await;
    assert_eq!(res.unwrap(), 42u32);

    // Short deadline -> operation times out
    let ctx2 = Context::with_deadline_from_now(std::time::Duration::from_millis(10));
    // Sleep longer than deadline inside the op
    let res2 = with_db_span(&ctx2, &attrs, async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok::<_, PersistenceError>(1u32)
    })
    .await;

    assert!(matches!(res2, Err(PersistenceError::DeadlineExceeded)));
}

#[test]
fn context_time_remaining_and_expiry() {
    let mut ctx = Context::new();
    assert!(!ctx.is_expired());

    ctx.set_deadline_from_now(std::time::Duration::from_millis(1));
    // busy-wait a tiny bit to let the deadline pass
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert!(ctx.is_expired());
    assert!(ctx.time_remaining().is_some());
}
