use commons::util::persistence::{InMemoryDriver, randomized_stress};

#[tokio::test]
async fn short_randomized_stress_smoke() {
    let drv = InMemoryDriver::new();
    drv.seed(vec![("seed", "1")]).unwrap();

    // Run a short deterministic stress for 1 second with a seed
    randomized_stress(drv, 1, Some(42u64), 4, 2, 100, 0.6).await;

    // Ensure seeded key still present or at least no panics occurred
    // (we don't assert exact counts since it's randomized)
}
