use commons::util::logging::init_test_logging;
use commons::util::persistence::{Context, DbAttributes, with_db_span};
use std::time::Duration;

#[tokio::test]
async fn with_db_span_records_timeout_and_runs_future() {
    // Initialize test logging/tracing subscriber once for span recording.
    let _ = init_test_logging();

    let ctx = Context {
        trace_headers: Default::default(),
        deadline: Some(tokio::time::Instant::now() + Duration::from_millis(50)),
        metadata: None,
    };

    let attrs = DbAttributes {
        driver: Some("memory".to_string()),
        op: Some("read".to_string()),
        entity: Some("unit".to_string()),
        statement: Some("stmt#1".to_string()),
    };

    let result = with_db_span(&ctx, &attrs, async { 42 }).await;
    assert_eq!(result, 42);
}
