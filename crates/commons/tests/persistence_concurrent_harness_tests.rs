use commons::util::persistence::{concurrent_readers, concurrent_writers, memory_driver_with_seed};

#[tokio::test]
async fn writers_and_readers_harness_smoke() {
    let drv = memory_driver_with_seed(&[("pre", "seed")]);

    // Spawn 8 writers each writing 50 entries
    concurrent_writers(drv.clone(), 8, 50, "k", "v").await;

    // Spawn 4 readers that each scan all keys
    let found = concurrent_readers(drv.clone(), 4, 8, 50, "k").await;

    // Each reader should find 8*50 keys; total across 4 readers is that * 4
    assert_eq!(found, 8 * 50 * 4);

    // pre-seeded key remains
    assert_eq!(drv.get("pre"), Some("seed".to_string()));
}
