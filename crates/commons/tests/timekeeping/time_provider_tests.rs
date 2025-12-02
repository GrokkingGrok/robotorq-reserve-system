//! Tests for TimeProvider implementations.
use std::time::{Duration, SystemTime};
use commons::util::timekeeping::{TimeProvider, SystemTimeProvider};

#[test]
fn system_time_provider_now_close_to_system_time() {
    let tp = SystemTimeProvider::new();
    let t1 = tp.now();
    let t2 = SystemTime::now();
    let diff = t2.duration_since(t1).unwrap_or_default();
    assert!(diff.as_millis() < 50, "SystemTimeProvider::now drift too large: {diff:?}");
}

#[tokio::test]
async fn system_time_provider_sleep_waits() {
    let tp = SystemTimeProvider::new();
    let start = std::time::Instant::now();
    tp.sleep(Duration::from_millis(30)).await;
    assert!(start.elapsed() >= Duration::from_millis(25));
}

#[cfg(feature = "sim")]
mod sim_tests {
    use super::*;
    use commons::util::timekeeping::{SimulatedTimeProvider};

    #[tokio::test]
    async fn simulated_time_provider_speedup_sleep() {
        let tp = SimulatedTimeProvider::new(10.0); // 10x faster
        let start_real = std::time::Instant::now();
        tp.sleep(Duration::from_millis(100)).await; // logical 100ms
        let real_elapsed = start_real.elapsed();
        assert!(real_elapsed.as_millis() < 30, "Expected accelerated sleep, got {:?}", real_elapsed);
    }
}
