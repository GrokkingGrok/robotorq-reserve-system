//! Concurrency tests for contract hash batch processing simulation.
//!
//! These tests model the scheduling and execution behavior of processing
//! multiple contract hash batches concurrently vs sequentially, ensuring:
//! * Parallel spawning reduces cumulative wall time (expected near single-task latency).
//! * Shared state mutation via `Mutex` remains race-free and complete.
//! * Lock acquisition ordering does not introduce deadlocks under mixed patterns.
//!
//! The real system spawns per-contract tasks during hash emission; here we
//! simulate processing time with `tokio::time::sleep` to avoid external I/O.
//! Tests enforce conservative speedup expectations that tolerate slower CI
//! hardware while still signaling regressions.
//!
//! Future improvements:
//! * Replace sleeps with instrumentation harness to measure actual hashing cost.
//! * Introduce property-based tests for lock ordering permutations.
//! * Add metrics snapshot assertions once exposed in test environment.

#[cfg(test)]
mod concurrent_tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    /// Simulate sequential processing: each contract awaits 100ms before continuing.
    async fn process_contracts_sequential(contract_ids: Vec<String>) -> Duration {
        let start = Instant::now();
        
        for contract_id in contract_ids {
            // Simulate 100ms processing per contract (hash retrieval, signing, publish)
            sleep(Duration::from_millis(100)).await;
            println!("Processed contract {} sequentially", contract_id);
        }
        
        start.elapsed()
    }

    /// Simulate concurrent processing: spawn all sleep tasks and await their completion.
    async fn process_contracts_concurrent(contract_ids: Vec<String>) -> Duration {
        let start = Instant::now();
        let mut tasks = vec![];
        
        for contract_id in contract_ids {
            let task = tokio::spawn(async move {
                // Simulate 100ms processing per contract
                sleep(Duration::from_millis(100)).await;
                println!("Processed contract {} concurrently", contract_id);
            });
            
            tasks.push(task);
        }
        
        // Wait for all contracts to finish
        for task in tasks {
            let _ = task.await;
        }
        
        start.elapsed()
    }

    #[tokio::test]
    async fn test_concurrent_processing_is_faster() {
        let contracts = vec![
            "contract-1".to_string(),
            "contract-2".to_string(),
            "contract-3".to_string(),
            "contract-4".to_string(),
            "contract-5".to_string(),
        ];

        // Sequential pass (~5 × 100ms ≈ 500ms)
        let sequential_time = process_contracts_sequential(contracts.clone()).await;
        println!("Sequential processing took: {:?}", sequential_time);
        
        // Concurrent pass (expect near 100–150ms on typical hardware)
        let concurrent_time = process_contracts_concurrent(contracts.clone()).await;
        println!("Concurrent processing took: {:?}", concurrent_time);
        
        // Assert ≥3× speedup (allows variance for scheduling overhead).
        assert!(
            concurrent_time < sequential_time / 3,
            "Concurrent processing should be at least 3x faster. Sequential: {:?}, Concurrent: {:?}",
            sequential_time,
            concurrent_time
        );
        
        // Ensure concurrent time hovers near single-contract duration (≤200ms).
        assert!(
            concurrent_time.as_millis() < 200,
            "Concurrent processing should complete in ~100ms, took {:?}",
            concurrent_time
        );
    }

    #[tokio::test]
    async fn test_concurrent_with_shared_state() {
        // Validate mutex-protected shared counter increments exactly once per task.
        let shared_counter = Arc::new(Mutex::new(0));
        let contract_ids = vec!["c1", "c2", "c3", "c4", "c5"];
        
        let mut tasks = vec![];
        
        for contract_id in contract_ids {
            let counter_clone = Arc::clone(&shared_counter);
            
            let task = tokio::spawn(async move {
                // Simulate processing
                sleep(Duration::from_millis(50)).await;
                
                    // Update shared state (analogous to contract mark_hash_send).
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
                
                println!("Contract {} incremented counter", contract_id);
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }
        
        // Check all increments applied; absence indicates lost update / race.
        let final_count = *shared_counter.lock().unwrap();
        assert_eq!(final_count, 5, "All 5 contracts should have incremented counter");
    }

    #[tokio::test]
    async fn test_no_deadlock_with_concurrent_locks() {
        // Ensure mixed lock acquisition ordering avoids deadlock by scoping guards.
        let state1 = Arc::new(Mutex::new(vec![1, 2, 3]));
        let state2 = Arc::new(Mutex::new(vec![4, 5, 6]));
        
        let mut tasks = vec![];
        
        for i in 0..10 {
            let s1 = Arc::clone(&state1);
            let s2 = Arc::clone(&state2);
            
            let task = tokio::spawn(async move {
                    // Properly scope mutex guards before awaited sleep to prevent holding across await.
                if i % 2 == 0 {
                    {
                        let _data1 = s1.lock().unwrap();
                        // Use data here
                    }
                    sleep(Duration::from_millis(10)).await;
                    {
                        let _data2 = s2.lock().unwrap();
                        // Use data here
                    }
                } else {
                    {
                        let _data2 = s2.lock().unwrap();
                        // Use data here
                    }
                    sleep(Duration::from_millis(10)).await;
                    {
                        let _data1 = s1.lock().unwrap();
                        // Use data here
                    }
                }
            });
            
            tasks.push(task);
        }
        
        // Test passes by completion; hanging indicates potential deadlock.
        let start = Instant::now();
        for task in tasks {
            let _ = task.await;
        }
        
        let elapsed = start.elapsed();
        println!("All tasks completed in {:?}", elapsed);
        
        // Completion should remain well under 2 seconds on commodity hardware.
        assert!(
            elapsed.as_secs() < 2,
            "Tasks should complete quickly without deadlock"
        );
    }
}
