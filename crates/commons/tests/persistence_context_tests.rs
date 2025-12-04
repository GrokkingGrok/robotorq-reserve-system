
use commons::util::persistence::Context;
use std::time::Duration;

#[tokio::test]
async fn deadline_and_time_remaining_behavior() {
    let mut ctx = Context::new();
    assert!(ctx.time_remaining().is_none());
    assert!(!ctx.is_expired());

    ctx.set_deadline_from_now(Duration::from_millis(50));
    let rem = ctx.time_remaining().unwrap();
    assert!(rem > Duration::from_millis(0));

    // Wait for the deadline to pass
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(ctx.is_expired());
    let rem_after = ctx.time_remaining().unwrap();
    assert_eq!(rem_after, Duration::from_millis(0));
}

#[test]
fn trace_header_insert_and_get() {
    let mut ctx = Context::new();
    ctx.insert_trace_header("traceparent", "00-abcdef-012345-01");
    let got = ctx.get_trace_header("traceparent").cloned();
    assert_eq!(got, Some("00-abcdef-012345-01".to_string()));
}
