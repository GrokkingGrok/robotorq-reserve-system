use std::time::{Duration, Instant};

use commons::util::logging::{init_test_logging, log_if_waited, spawn_traced};

#[test]
fn init_test_logging_idempotent() {
    // Should succeed the first time and be a no-op on subsequent calls.
    init_test_logging("debug").unwrap();
    init_test_logging("debug").unwrap();
}

#[test]
fn log_if_waited_exercises_both_branches() {
    // Exceed threshold to hit warn path
    let earlier = Instant::now() - Duration::from_millis(10);
    log_if_waited("a", earlier, Duration::from_millis(1));

    // Stay below threshold to hit debug path
    let now = Instant::now();
    log_if_waited("b", now, Duration::from_secs(1));
}

#[tokio::test]
async fn spawn_traced_runs_future() {
    let handle = spawn_traced("unit", async move { 42u32 });
    let v = handle.await.expect("join");
    assert_eq!(v, 42);
}
